//! Memoria de vídeo con write-combining. El firmware del portátil la deja sin caché (ningún
//! MTRR la cubre y el tipo por defecto es UC): pasar un fotograma de 8 MB tarda 34 ms. Con un
//! MTRR variable libre de tipo WC las escrituras se juntan y va mucho más rápido.
//! Solo se usa un MTRR que estaba libre, y se vuelve a dejar libre antes de abrir cualquier
//! cosa. Pasos de la SDM de Intel (vol. 3A, «MemTypeSet») y del APM de AMD: sin interrupciones,
//! caché apagada y vaciada, TLB vaciada y MTRR desactivados mientras se escriben.

use core::arch::asm;
use core::arch::x86_64::__cpuid;

use uefi::boot::{self, Tpl};

const MTRRCAP: u32 = 0xFE;
const DEF_TYPE: u32 = 0x2FF;
/// IA32_MTRR_PHYSBASE0; la máscara de cada uno va justo detrás
const BASE0: u32 = 0x200;
const TIPO_WC: u64 = 1;
const VALIDO: u64 = 1 << 11;

/// El MTRR que se ha puesto, para quitarlo
pub struct Wc {
    reg: u32,
}

pub enum Fallo {
    /// La CPU no tiene MTRR o no admite WC
    SinMtrr,
    /// La zona se solapa con un MTRR del firmware (si fuera UC, ganaría él)
    Solapa,
    SinLibre,
}

unsafe fn leer(msr: u32) -> u64 {
    let (lo, hi): (u32, u32);
    // SAFETY: quien llama sabe que el MSR existe.
    unsafe { asm!("rdmsr", in("ecx") msr, out("eax") lo, out("edx") hi, options(nomem, nostack, preserves_flags)) };
    ((hi as u64) << 32) | lo as u64
}

unsafe fn escribir(msr: u32, v: u64) {
    // SAFETY: quien llama sabe que el MSR existe y que el valor es válido.
    unsafe { asm!("wrmsr", in("ecx") msr, in("eax") v as u32, in("edx") (v >> 32) as u32, options(nostack, preserves_flags)) };
}

/// Bits de dirección física de la CPU
fn bits_fisicos() -> u32 {
    let max = __cpuid(0x8000_0000).eax;
    if max >= 0x8000_0008 { __cpuid(0x8000_0008).eax & 0xFF } else { 36 }
}

/// Vacía toda la TLB (con las páginas globales ya desactivadas)
unsafe fn vaciar_tlb() {
    // SAFETY: recargar CR3 con su mismo valor no cambia nada más.
    unsafe { asm!("mov {0}, cr3", "mov cr3, {0}", out(reg) _, options(nostack, preserves_flags)) };
}

/// Cambia MTRR con el procedimiento de la SDM
unsafe fn cambiar(f: impl FnOnce()) {
    // SAFETY: sin interrupciones durante el cambio; el guardián devuelve el TPL al soltarse.
    let _tpl = unsafe { boot::raise_tpl(Tpl::HIGH_LEVEL) };
    // SAFETY: en el modo de arranque de UEFI estamos en el anillo 0; CR0 y CR4 vuelven a su valor.
    unsafe {
        let (cr0, cr4): (u64, u64);
        asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
        asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
        // caché apagada (CD = 1, NW = 0) y vaciada
        asm!("mov cr0, {}", in(reg) (cr0 | (1 << 30)) & !(1 << 29), options(nostack, preserves_flags));
        asm!("wbinvd", options(nostack, preserves_flags));
        // sin páginas globales: así recargar CR3 vacía toda la TLB
        asm!("mov cr4, {}", in(reg) cr4 & !(1 << 7), options(nostack, preserves_flags));
        vaciar_tlb();
        let def = leer(DEF_TYPE);
        escribir(DEF_TYPE, def & !(1 << 11));
        f();
        asm!("wbinvd", options(nostack, preserves_flags));
        vaciar_tlb();
        escribir(DEF_TYPE, def);
        asm!("mov cr0, {}", in(reg) cr0, options(nostack, preserves_flags));
        asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));
    }
}

/// Pone en WC la zona [fb, fb + tam) (la más pequeña de tamaño potencia de dos y alineada que
/// la cubre: en el portátil, dentro de la BAR prefetchable de la gráfica) con un MTRR libre
pub fn activar(fb: u64, tam: u64) -> Result<Wc, Fallo> {
    if __cpuid(1).edx & (1 << 12) == 0 {
        return Err(Fallo::SinMtrr);
    }
    // SAFETY: con MTRR, IA32_MTRRCAP existe.
    let cap = unsafe { leer(MTRRCAP) };
    if cap & (1 << 10) == 0 {
        return Err(Fallo::SinMtrr);
    }
    let fisica = (1u64 << bits_fisicos()) - 1;
    let mut largo = tam.next_power_of_two().max(4096);
    let mut base = fb & !(largo - 1);
    while base + largo < fb + tam {
        largo *= 2;
        base = fb & !(largo - 1);
    }
    let mut libre = None;
    for n in 0..(cap & 0xFF) as u32 {
        // SAFETY: hay VCNT pares de MTRR variables.
        let (b, m) = unsafe { (leer(BASE0 + 2 * n), leer(BASE0 + 2 * n + 1)) };
        if m & VALIDO == 0 {
            libre = libre.or(Some(n));
            continue;
        }
        let mascara = m & fisica & !0xFFF;
        let (inicio, fin) = (b & mascara, (b & mascara) + (!mascara & fisica) + 1);
        if inicio < base + largo && base < fin {
            return Err(Fallo::Solapa);
        }
    }
    let reg = libre.ok_or(Fallo::SinLibre)?;
    let mascara = (!(largo - 1) & fisica & !0xFFF) | VALIDO;
    // SAFETY: el par estaba libre y la zona no se solapa con ningún otro.
    unsafe {
        cambiar(|| {
            escribir(BASE0 + 2 * reg, base | TIPO_WC);
            escribir(BASE0 + 2 * reg + 1, mascara);
        })
    };
    Ok(Wc { reg })
}

impl Wc {
    /// Deja el MTRR libre otra vez, como estaba
    pub fn quitar(self) {
        // SAFETY: es el par que puso activar; vuelve a quedar sin usar.
        unsafe {
            cambiar(|| {
                escribir(BASE0 + 2 * self.reg + 1, 0);
                escribir(BASE0 + 2 * self.reg, 0);
            })
        };
    }
}

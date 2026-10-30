//! Menú de arranque propio del portátil: Windows en el centro (por defecto, arranca sola a
//! los 10 s), Kali a la izquierda y Arch a la derecha, con sus animaciones; Esc abre el
//! rescate (Limine), rEFInd, la BIOS, reiniciar y apagar. Diseño: maqueta/.
#![no_main]
#![no_std]

extern crate alloc;

mod arranque;
mod escena;
mod grafico;
mod mates;
mod recursos;
mod video;

use core::time::Duration;

use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol};
use uefi::prelude::*;
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput, PixelFormat};
use uefi::proto::console::text::{Key, ScanCode};
use uefi::system;

use escena::{Destino, Escena, Tecla, Video, FONDO, H, W};
use grafico::{Capa, Lienzo};

fn tsc() -> u64 {
    // SAFETY: RDTSC existe en todo x86_64.
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// Ciclos del TSC por segundo, medidos contra el reloj del firmware
fn tsc_hz() -> f64 {
    let a = tsc();
    boot::stall(Duration::from_millis(50));
    (tsc() - a) as f64 * 20.0
}

fn abrir_gop() -> uefi::Result<ScopedProtocol<GraphicsOutput>> {
    let handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
    // GetProtocol y no exclusivo: así la consola de texto sigue conectada para lo que se arranque
    // SAFETY: nadie cierra la GOP mientras el menú la usa.
    let mut gop = unsafe {
        boot::open_protocol::<GraphicsOutput>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )?
    };
    let actual = gop.current_mode_info().resolution();
    if actual != (W as usize, H as usize) {
        if let Some(modo) = gop.modes().find(|m| m.info().resolution() == (W as usize, H as usize)) {
            gop.set_mode(&modo)?;
        }
    }
    Ok(gop)
}

/// Copia el lienzo a la pantalla, centrado (o recortado si la pantalla es más pequeña).
/// Si la memoria de vídeo es BGR de 32 bits (lo normal) se escribe directamente, que en
/// muchos firmwares es bastante más rápido que Blt.
fn presentar(gop: &mut GraphicsOutput, lz: &Lienzo) {
    let info = gop.current_mode_info();
    let (sw, sh) = info.resolution();
    let (w, h) = (sw.min(lz.w), sh.min(lz.h));
    let (ox, oy) = ((lz.w - w) / 2, (lz.h - h) / 2);
    let (dx, dy) = ((sw - w) / 2, (sh - h) / 2);
    if info.pixel_format() == PixelFormat::Bgr {
        let paso = info.stride();
        let mut fb = gop.frame_buffer();
        if fb.size() >= paso * sh * 4 {
            let base = fb.as_mut_ptr().cast::<u32>();
            for y in 0..h {
                let origen = &lz.px[(oy + y) * lz.w + ox..(oy + y) * lz.w + ox + w];
                // SAFETY: la fila cabe en la memoria de vídeo (comprobado su tamaño arriba).
                unsafe { core::ptr::copy_nonoverlapping(origen.as_ptr(), base.add((dy + y) * paso + dx), w) };
            }
            return;
        }
    }
    // SAFETY: BltPixel son 4 bytes (azul, verde, rojo, nada), lo mismo que 0x00RRGGBB en little endian.
    let px: &[BltPixel] = unsafe { core::slice::from_raw_parts(lz.px.as_ptr().cast(), lz.px.len()) };
    let _ = gop.blt(BltOp::BufferToVideo {
        buffer: px,
        src: BltRegion::SubRectangle { coords: (ox, oy), px_stride: lz.w },
        dest: (dx, dy),
        dims: (w, h),
    });
}

fn traducir(k: Key) -> Option<Tecla> {
    match k {
        Key::Special(ScanCode::LEFT) => Some(Tecla::Izquierda),
        Key::Special(ScanCode::RIGHT) => Some(Tecla::Derecha),
        Key::Special(ScanCode::UP) => Some(Tecla::Arriba),
        Key::Special(ScanCode::DOWN) => Some(Tecla::Abajo),
        Key::Special(ScanCode::ESCAPE) => Some(Tecla::Esc),
        Key::Printable(c) => match u16::from(c) {
            13 => Some(Tecla::Intro),
            0x66 | 0x46 => Some(Tecla::Fps), // f: fotogramas por segundo
            _ => None,
        },
        _ => None,
    }
}

/// Pone el write-combining en la memoria de vídeo (si no lo tiene ya) y se lo dice a la escena
fn poner_wc(gop: Option<&mut ScopedProtocol<GraphicsOutput>>, wc: &mut Option<video::Wc>, escena: &mut Escena) {
    let Some(g) = gop else { return };
    if wc.is_some() {
        return;
    }
    let mut fb = g.frame_buffer();
    let (base, tam) = (fb.as_mut_ptr() as u64, fb.size() as u64);
    match video::activar(base, tam) {
        Ok(w) => {
            *wc = Some(w);
            escena.video(Video::Wc);
        }
        Err(_) => escena.video(Video::Fallo),
    }
}

/// Anima el menú hasta que se elige algo y su animación termina
fn bucle(escena: &mut Escena, lz: &mut Lienzo, capa: &mut Capa, hz: f64, wc: &mut Option<video::Wc>) -> Destino {
    let mut gop = abrir_gop().ok();
    poner_wc(gop.as_mut(), wc, escena);
    // como mucho 60 fotogramas por segundo: la pantalla no da más
    let periodo = (hz / 60.0) as u64;
    let mut previo = tsc();
    // para la tecla F: fotogramas y ciclos de cada parte en el último medio segundo
    let (mut n, mut ciclos, mut desde) = (0u32, [0u64; 3], previo);
    loop {
        let ahora = tsc();
        let dt = ((ahora - previo) as f64 / hz) as f32;
        previo = ahora;
        system::with_stdin(|entrada| {
            while let Ok(Some(k)) = entrada.read_key() {
                if let Some(t) = traducir(k) {
                    escena.tecla(t);
                }
            }
        });
        let t1 = tsc();
        if let Some(d) = escena.actualizar(dt.min(0.05)) {
            return d;
        }
        escena.pintar(lz, capa);
        let t2 = tsc();
        if let Some(g) = gop.as_mut() {
            presentar(g, lz);
        }
        let t3 = tsc();
        n += 1;
        ciclos[0] += t2 - t1;
        ciclos[1] += t3 - t2;
        ciclos[2] += t1 - ahora;
        if (t3 - desde) as f64 > hz / 2.0 {
            let ms = |c: u64| (c as f64 / n as f64 / hz * 1000.0 + 0.5) as u32;
            escena.medir([(n as f64 * hz / (t3 - desde) as f64 + 0.5) as u32, ms(ciclos[0]), ms(ciclos[1]), ms(ciclos[2])]);
            (n, ciclos, desde) = (0, [0; 3], t3);
        }
        while tsc() - ahora < periodo {
            core::hint::spin_loop();
        }
    }
}

#[entry]
fn main() -> Status {
    // sin perro guardián: el menú puede quedarse esperando lo que haga falta
    let _ = boot::set_watchdog_timer(0, 0x10000, None);
    let conf = arranque::Conf::leer();
    let hz = tsc_hz();
    let mut escena = Escena::new(tsc(), conf.espera);
    let mut lz = Lienzo::new(W as usize, H as usize);
    let mut capa = Capa::new((W * FONDO) as usize, (H * FONDO) as usize);
    let mut wc = None;
    loop {
        let d = bucle(&mut escena, &mut lz, &mut capa, hz, &mut wc);
        // lo que se abra encuentra los MTRR como los dejó el firmware
        if let Some(w) = wc.take() {
            w.quitar();
            escena.video(Video::SinCache);
        }
        match arranque::ejecutar(d, &conf) {
            Ok(()) => escena.volver(),
            Err(_) => escena.fallo(d),
        }
    }
}

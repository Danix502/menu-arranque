//! Qué abre cada opción: lo lee de `\EFI\menu-arranque\menu.conf` (en la misma partición
//! EFI que el menú) y lo arranca con LoadImage + StartImage.

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use uefi::boot::{self, LoadImageSource, OpenProtocolAttributes, OpenProtocolParams};
use uefi::fs::{FileSystem, Path};
use uefi::proto::device_path::build::{self, DevicePathBuilder};
use uefi::proto::device_path::DevicePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::BootPolicy;
use uefi::runtime::{self, ResetType, VariableAttributes, VariableVendor};
use uefi::{cstr16, CString16, Status};

use crate::escena::{Destino, Sis};

pub const RUTA_CONF: &uefi::CStr16 = cstr16!("\\EFI\\menu-arranque\\menu.conf");

pub struct Conf {
    pub espera: f32,
    windows: String,
    arch: String,
    arch_opciones: String,
    kali: String,
    kali_opciones: String,
    rescate: String,
    refind: String,
}

impl Conf {
    pub fn leer() -> Conf {
        let mut c = Conf {
            espera: 10.0,
            windows: "\\EFI\\Microsoft\\Boot\\bootmgfw.efi".to_string(),
            arch: String::new(),
            arch_opciones: String::new(),
            kali: String::new(),
            kali_opciones: String::new(),
            rescate: "\\EFI\\limine\\BOOTX64.EFI".to_string(),
            refind: "\\EFI\\refind\\refind_x64.efi".to_string(),
        };
        let Ok(proto) = boot::get_image_file_system(boot::image_handle()) else { return c };
        let Ok(texto) = FileSystem::new(proto).read_to_string(Path::new(RUTA_CONF)) else { return c };
        for linea in texto.lines() {
            let linea = linea.trim();
            if linea.is_empty() || linea.starts_with('#') {
                continue;
            }
            let Some((clave, valor)) = linea.split_once('=') else { continue };
            let valor = valor.trim().to_string();
            match clave.trim() {
                "espera" => c.espera = valor.parse::<f32>().unwrap_or(10.0).clamp(1.0, 60.0),
                "windows" => c.windows = valor,
                "arch" => c.arch = valor,
                "arch_opciones" => c.arch_opciones = valor,
                "kali" => c.kali = valor,
                "kali_opciones" => c.kali_opciones = valor,
                "rescate" => c.rescate = valor,
                "refind" => c.refind = valor,
                _ => {}
            }
        }
        c
    }
}

/// Abre el destino. Solo vuelve si falla o si lo abierto termina y devuelve el control.
pub fn ejecutar(d: Destino, c: &Conf) -> uefi::Result {
    match d {
        Destino::Sistema(Sis::Windows) => lanzar(&c.windows, ""),
        Destino::Sistema(Sis::Arch) => lanzar(&c.arch, &c.arch_opciones),
        Destino::Sistema(Sis::Kali) => lanzar(&c.kali, &c.kali_opciones),
        Destino::Rescate => lanzar(&c.rescate, ""),
        Destino::Refind => lanzar(&c.refind, ""),
        Destino::Bios => bios(),
        Destino::Reiniciar => runtime::reset(ResetType::COLD, Status::SUCCESS, None),
        Destino::Apagar => runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None),
    }
}

/// Carga un .efi (o un kernel con EFI stub) de la partición del menú y lo arranca con sus
/// opciones (en Linux, la línea del kernel con `initrd=`).
fn lanzar(ruta: &str, opciones: &str) -> uefi::Result {
    if ruta.is_empty() {
        return Err(Status::NOT_FOUND.into());
    }
    let yo = boot::image_handle();
    let disco = boot::open_protocol_exclusive::<LoadedImage>(yo)?.device().ok_or(Status::NOT_FOUND)?;
    // ruta completa = la de la partición + el fichero
    let imagen = {
        // SAFETY: solo se lee la ruta del dispositivo; GetProtocol no impide que otros la usen.
        let ruta_disco = unsafe {
            boot::open_protocol::<DevicePath>(
                OpenProtocolParams { handle: disco, agent: yo, controller: None },
                OpenProtocolAttributes::GetProtocol,
            )?
        };
        let fichero = CString16::try_from(ruta).map_err(|_| Status::INVALID_PARAMETER)?;
        let mut buf = Vec::new();
        let mut b = DevicePathBuilder::with_vec(&mut buf);
        for nodo in ruta_disco.node_iter() {
            b = b.push(&nodo).map_err(|_| Status::BUFFER_TOO_SMALL)?;
        }
        b = b.push(&build::media::FilePath { path_name: &fichero }).map_err(|_| Status::BUFFER_TOO_SMALL)?;
        let completa = b.finalize().map_err(|_| Status::BUFFER_TOO_SMALL)?;
        boot::load_image(yo, LoadImageSource::FromDevicePath { device_path: completa, boot_policy: BootPolicy::ExactMatch })?
    };
    if !opciones.is_empty() {
        let texto = CString16::try_from(opciones).map_err(|_| Status::INVALID_PARAMETER)?;
        // tiene que seguir viva mientras corre la imagen: se suelta a propósito
        let texto: &'static CString16 = Box::leak(Box::new(texto));
        let mut li = boot::open_protocol_exclusive::<LoadedImage>(imagen)?;
        // SAFETY: el texto vive para siempre y el tamaño es el suyo en bytes, con el nulo.
        unsafe { li.set_load_options(texto.as_ptr().cast(), texto.num_bytes() as u32) };
    }
    boot::start_image(imagen)
}

/// Pide al firmware que el próximo arranque entre en su configuración y reinicia
fn bios() -> uefi::Result {
    let mut buf = [0u8; 8];
    let (datos, _) = runtime::get_variable(cstr16!("OsIndicationsSupported"), &VariableVendor::GLOBAL_VARIABLE, &mut buf)
        .map_err(|e| e.status())?;
    let soportado = u64::from_le_bytes(buf_a_8(datos));
    if soportado & 1 == 0 {
        return Err(Status::UNSUPPORTED.into());
    }
    let mut actual = [0u8; 8];
    let previo = runtime::get_variable(cstr16!("OsIndications"), &VariableVendor::GLOBAL_VARIABLE, &mut actual)
        .map(|(d, _)| u64::from_le_bytes(buf_a_8(d)))
        .unwrap_or(0);
    let atributos = VariableAttributes::NON_VOLATILE | VariableAttributes::BOOTSERVICE_ACCESS | VariableAttributes::RUNTIME_ACCESS;
    runtime::set_variable(cstr16!("OsIndications"), &VariableVendor::GLOBAL_VARIABLE, atributos, &(previo | 1).to_le_bytes())?;
    runtime::reset(ResetType::COLD, Status::SUCCESS, None)
}

fn buf_a_8(d: &[u8]) -> [u8; 8] {
    let mut b = [0u8; 8];
    let n = d.len().min(8);
    b[..n].copy_from_slice(&d[..n]);
    b
}

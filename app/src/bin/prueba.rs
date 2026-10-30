//! Destino de prueba para QEMU: escribe por la consola (y el puerto serie) las opciones con
//! que lo arrancaron, pinta la pantalla de verde y vuelve al menú a los 3 s.
#![no_main]
#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::fmt::Write;
use core::time::Duration;

use uefi::boot;
use uefi::prelude::*;
use uefi::proto::console::gop::{BltOp, BltPixel, GraphicsOutput};
use uefi::proto::loaded_image::LoadedImage;
use uefi::system;

#[entry]
fn main() -> Status {
    let opciones = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())
        .ok()
        .and_then(|li| li.load_options_as_cstr16().ok().map(|s| format!("{s}")))
        .unwrap_or_else(|| String::from("(ninguna)"));
    system::with_stdout(|o| {
        let _ = writeln!(o, "PRUEBA-ARRANCADA opciones=[{opciones}]");
    });
    if let Ok(h) = boot::get_handle_for_protocol::<GraphicsOutput>() {
        if let Ok(mut gop) = boot::open_protocol_exclusive::<GraphicsOutput>(h) {
            let (w, h) = gop.current_mode_info().resolution();
            let _ = gop.blt(BltOp::VideoFill { color: BltPixel::new(0, 160, 60), dest: (0, 0), dims: (w, h) });
        }
    }
    boot::stall(Duration::from_secs(3));
    Status::SUCCESS
}

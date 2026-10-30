extern crate alloc;

#[path = "../../app/src/escena.rs"]
mod escena;
#[path = "../../app/src/grafico.rs"]
mod grafico;
#[path = "../../app/src/mates.rs"]
mod mates;
#[path = "../../app/src/recursos.rs"]
mod recursos;

use std::time::Instant;

use escena::{Escena, Tecla, FONDO, H, W};
use grafico::{Capa, Lienzo};

fn medir(nombre: &str, e: &mut Escena, lz: &mut Lienzo, capa: &mut Capa) {
    let n = 240;
    let t = Instant::now();
    for _ in 0..n {
        e.actualizar(1.0 / 60.0);
        e.pintar(lz, capa);
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    println!("{nombre:<22} {ms:6.2} ms/fotograma  ({:.0} fps)", 1000.0 / ms);
}

fn main() {
    if std::env::args().any(|a| a == "piezas") { piezas(); return; }
    if let Some(carpeta) = std::env::args().skip_while(|a| a != "fotos").nth(1) { fotos(&carpeta); return; }
    let mut e = Escena::new(12345, 10.0);
    let mut lz = Lienzo::new(W as usize, H as usize);
    let mut capa = Capa::new((W * FONDO) as usize, (H * FONDO) as usize);
    medir("Windows (inicio)", &mut e, &mut lz, &mut capa);
    e.tecla(Tecla::Izquierda);
    medir("Kali (glitch)", &mut e, &mut lz, &mut capa);
    e.tecla(Tecla::Derecha);
    e.tecla(Tecla::Derecha);
    medir("Arch (fuego nublado)", &mut e, &mut lz, &mut capa);
    e.tecla(Tecla::Esc);
    medir("Menú de Esc", &mut e, &mut lz, &mut capa);
    // la animación de arrancar cada sistema (su primer segundo, lo más caro)
    for (nombre, teclas) in [("Lanzar Kali", &[Tecla::Izquierda][..]), ("Lanzar Windows", &[][..]), ("Lanzar Arch", &[Tecla::Derecha][..])] {
        let mut e = Escena::new(12345, 10.0);
        for _ in 0..60 {
            e.actualizar(1.0 / 60.0);
        }
        for &t in teclas {
            e.tecla(t);
        }
        for _ in 0..90 {
            e.actualizar(1.0 / 60.0);
        }
        e.tecla(Tecla::Intro);
        let n = 60;
        let i = Instant::now();
        for _ in 0..n {
            e.actualizar(1.0 / 60.0);
            e.pintar(&mut lz, &mut capa);
        }
        let ms = i.elapsed().as_secs_f64() * 1000.0 / n as f64;
        println!("{nombre:<22} {ms:6.2} ms/fotograma  ({:.0} fps)", 1000.0 / ms);
    }
}

#[allow(dead_code)]
pub fn piezas() {
    use grafico::Mezcla;
    let mut lz = Lienzo::new(W as usize, H as usize);
    let mut capa = Capa::new((W * FONDO) as usize, (H * FONDO) as usize);
    let n = 200;
    let mut t = |nombre: &str, f: &mut dyn FnMut()| {
        let i = Instant::now();
        for _ in 0..n { f(); }
        println!("  {nombre:<28} {:6.2} ms", i.elapsed().as_secs_f64() * 1000.0 / n as f64);
    };
    t("capa.limpiar", &mut || capa.limpiar());
    t("humo x8 (girada)", &mut || for i in 0..8 { capa.girada(&recursos::HUMO_0, 100.0 + i as f32 * 50.0, 130.0, 2.7 * 2.0 * 0.25, 0.3, [150.0, 8.0, 30.0], 0.6) });
    t("aura x1 (girada)", &mut || capa.girada(&recursos::AURA_WINDOWS, 240.0, 113.0, 0.25, 0.0, [30.0, 110.0, 255.0], 0.9));
    t("ampliar", &mut || lz.ampliar(&mut capa));
    t("logo 320 normal", &mut || lz.mascara(&recursos::LOGO_WINDOWS, recursos::LOGO_WINDOWS.entero(), 800.0, 300.0, 1.0, [14.0, 128.0, 224.0], 1.0, Mezcla::Normal));
    t("logo 320 esc .74", &mut || lz.mascara(&recursos::LOGO_WINDOWS, recursos::LOGO_WINDOWS.entero(), 800.0, 300.0, 0.74, [14.0, 128.0, 224.0], 0.42, Mezcla::Normal));
    t("lineas kali", &mut || lz.lineas(&recursos::LOGO_KALI, 300.0, 300.0, 1.0, 1.0, 100.0, [255.0, 120.0, 135.0], 0.8));
    t("oscurecer", &mut || lz.oscurecer(0.5));
    t("panel + borde", &mut || { lz.mascara(&recursos::PANEL, recursos::PANEL.entero(), 600.0, 300.0, 1.0, [10.0, 8.0, 14.0], 0.9, Mezcla::Normal); lz.mascara(&recursos::PANEL_BORDE, recursos::PANEL.entero(), 600.0, 300.0, 1.0, [255.0; 3], 0.08, Mezcla::Normal) });
    t("texto nombre", &mut || lz.texto(&recursos::NOMBRE_WINDOWS, 900.0, 600.0, [236.0, 232.0, 242.0], 1.0));
}

/// Fotogramas al lanzar Arch en t = 0,4, 0,8 y 1,2 s (PPM), para compararlos con la maqueta
pub fn fotos(carpeta: &str) {
    let mut lz = Lienzo::new(W as usize, H as usize);
    let mut capa = Capa::new((W * FONDO) as usize, (H * FONDO) as usize);
    let mut e = Escena::new(12345, 10.0);
    for _ in 0..60 { e.actualizar(1.0 / 60.0); }
    e.tecla(Tecla::Derecha);
    for _ in 0..150 { e.actualizar(1.0 / 60.0); }
    e.tecla(Tecla::Intro);
    for n in 1..=72 {
        e.actualizar(1.0 / 60.0);
        if n % 24 == 0 {
            e.pintar(&mut lz, &mut capa);
            let mut ppm = format!("P6 {} {} 255
", lz.w, lz.h).into_bytes();
            for p in &lz.px { ppm.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]); }
            std::fs::write(format!("{carpeta}/app-{}.ppm", n * 10 / 6), ppm).unwrap();
        }
    }
}

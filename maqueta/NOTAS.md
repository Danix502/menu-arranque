# Maqueta del menú de arranque

Menú propio que sustituirá a rEFInd como primera pantalla del portátil (1920×1080). Se abre con doble clic en `index.html`.

- `←` `→` elegir · `Intro` arrancar · `Esc` más opciones · `R` volver a empezar · `F` pantalla completa · `H` oculta el panel.
- Windows en el centro y elegida por defecto; si no se toca nada, arranca sola a los **10 s** (cualquier tecla para la cuenta). Lo demás (Rescate con Limine, rEFInd, BIOS, reiniciar, apagar) solo sale con `Esc`.

## Diseño elegido (2026-10-26)

| | Efecto |
|---|---|
| Windows (azul) | Aura azul suave y separada del logo; los cuatro cuadros laten por turnos |
| Arch (lila `#9b5cff`) | Fuego nublado (2026-10-26, sobre `fuego.html`): llamas cortas pegadas al borde que alimentan una nube morada que sube y se enrosca. Prende poco a poco al elegirlo (1,6 s) y la nube no pasa por encima del logo (2026-10-27) |
| Kali (rojo `#c0182f`) | Glitch: cortes, fantasmas rojo y púrpura, líneas y barrido |
| Fondo | Negro con humo rojo y morado que deriva despacio por los bordes; sin pista de teclas abajo |
| Nombres y Esc | «ARCH» y «KALI» sin LINUX; en Esc, «Rescate» a secas y sin descripciones (2026-10-27) |
| Al lanzar (2026-10-27) | Kali: toda la pantalla se glitchea y se apaga como una tele vieja (1,35 s). Windows: los cuatro cristales se abren hacia las esquinas mientras se entra en la luz de detrás (1,45 s). Arch: la pantalla arde como un papel desde la A, con el borde encendido y negro detrás (1,8 s; el fuego que subía desde abajo «se veía poco natural»). Panel «Al arrancar» |
| Letras (2026-10-27, `letras.html`) | Kali VT323, Windows Selawik Semibold, Arch Exo 2 a 600 (todas OFL). Descartadas: JetBrains Mono, Share Tech Mono, Selawik Light, Open Sans, Jost y Space Grotesk |

Implantado en `app/` (programa UEFI). Descartado: la llama de lenguas sueltas (2026-10-26: quedaba artificial; en `fuego.html` se comparó con una llama de ruido y un fuego que se propaga celda a celda), unas brasas y la ceniza de fondo (con la llama de Arch, demasiado fuego), las chispas de Arch (parecían burbujas) y el aura de Windows con mucho brillo (se juntaba con el azul del logo). El panel aún deja comparar los otros efectos.

## Paso a la app UEFI
- Todo lo caro (degradados, desenfoques, ruido del humo, glifos de la Nerd Font, textos) se hace una vez: en la app, con Pillow al compilar, y entra en el binario como máscaras.
- El fuego de Arch es una capa de celdas de 2 px: por celda, búsquedas en dos texturas de ruido, cuentas con enteros y dos paletas; se agranda ×2 y se suma (texturas, envolventes y paletas, de Pillow).
- En cada fotograma solo hay: copiar sprites con escala, giro y alfa (mezcla normal o aditiva), franjas de un sprite (glitch) y recortar una franja con la silueta de un logo (barrido de Kali).
- El papel de Arch es otra capa de ¼ que va encima de todo: por celda, cuánto negro y el color que se suma; la distancia de cada celda a la A, con su borde irregular, se calcula una vez al empezar.
- El fondo es muy suave: se dibuja a un cuarto de resolución y se agranda, o se refresca a menos fotogramas que los logos.
- Logos: Arch `U+F303` (sin el ™) y Kali `U+F327` de `recursos/SymbolsNerdFont-Regular.ttf` (MIT); Windows son cuatro cuadros. Texto: Alegreya Sans (OFL).

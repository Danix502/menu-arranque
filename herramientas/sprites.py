"""Dibuja de antemano todo lo caro del menú (logos, auras, humo, fuego de Arch, la luz de Windows,
textos y el panel de Esc) a juego con la maqueta, y lo deja para el programa UEFI:
  app/src/recursos.bin  los píxeles, uno detrás de otro
  app/src/recursos.rs   dónde empieza cada sprite, su tamaño y su ancla
Máscaras: 1 byte de alfa por píxel; el color lo pone el programa.
Uso: python herramientas/sprites.py (antes de compilar)."""
from math import ceil
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

RAIZ = Path(__file__).resolve().parent.parent
REC = RAIZ / "recursos"
SIMBOLOS = str(REC / "SymbolsNerdFont-Regular.ttf")
TEXTO = str(REC / "alegreya-sans-normal-500-latin.woff2")
L, A = 320, 760  # lienzo de un logo y de su aura

# ------------------------------------------------------------------ utilidades


def a_float(im):
    return np.asarray(im, dtype=np.float32) / 255


def a_mascara(arr):
    return Image.fromarray(np.clip(np.rint(arr * 255), 0, 255).astype(np.uint8), "L")


def difuminar(arr, sigma):
    return a_float(a_mascara(arr).filter(ImageFilter.GaussianBlur(sigma)))


def radial(tam, paradas):
    """Alfa de un degradado radial centrado (posición 0..1 del radio → alfa)."""
    yy, xx = np.mgrid[0:tam, 0:tam].astype(np.float32) + .5
    r = np.hypot(xx - tam / 2, yy - tam / 2) / (tam / 2)
    return np.interp(r, [p for p, _ in paradas], [a for _, a in paradas]).astype(np.float32)


def sobre(*capas):
    """Varias capas de alfa del mismo color, una encima de otra."""
    resto = np.ones_like(capas[0])
    for c in capas:
        resto *= 1 - c
    return 1 - resto


# ------------------------------------------------------------------ logos

def logo_glifo(glifo, caja, ancho=1.0):
    """Glifo de la Nerd Font centrado en un lienzo L×L, encajado en caja (w, h)."""
    f = ImageFont.truetype(SIMBOLOS, 1600)
    grande = Image.new("L", (3000, 2600))
    ImageDraw.Draw(grande).text((200, 2200), glifo, font=f, fill=255, anchor="ls")
    x0, y0, x1, y1 = grande.getbbox()
    x1 = x0 + round((x1 - x0) * ancho)  # el ™ de Arch queda fuera
    tinta = grande.crop((x0, y0, x1, y1))
    k = min(caja[0] / tinta.width, caja[1] / tinta.height)
    w, h = tinta.width * k, tinta.height * k
    # remuestreo a 4× y luego a 1×: bordes suaves y posición con decimales
    lienzo = Image.new("L", (L * 4, L * 4))
    pequeña = tinta.resize((round(w * 4), round(h * 4)), Image.LANCZOS)
    lienzo.paste(pequeña, (round((L * 4 - pequeña.width) / 2), round((L * 4 - pequeña.height) / 2)))
    return a_float(lienzo.resize((L, L), Image.LANCZOS))


LADO, HUECO = 184, 9
CUADRO = (LADO - HUECO) / 2
X0 = (L - LADO) / 2
CUADROS = [(X0 + i * (CUADRO + HUECO), X0 + j * (CUADRO + HUECO), CUADRO, CUADRO) for i, j in [(0, 0), (1, 0), (1, 1), (0, 1)]]


def logo_windows():
    im = Image.new("L", (L * 4, L * 4))
    d = ImageDraw.Draw(im)
    for x, y, w, h in CUADROS:
        d.rectangle([x * 4, y * 4, (x + w) * 4 - 1, (y + h) * 4 - 1], fill=255)
    return a_float(im.resize((L, L), Image.BOX))


LOGOS = {
    "windows": logo_windows(),
    "arch": logo_glifo("", (206, 196), 1944 / 2048),
    "kali": logo_glifo("", (262, 196)),
}
# brillo del aura: núcleo redondo, silueta muy difuminada, silueta pegada al logo
BRILLO = {"windows": (.2, .6, .12), "arch": (.30, .95, .8), "kali": (.30, .95, .8)}


def aura(id):
    nucleo, ancha, cerca = BRILLO[id]
    grad = radial(A, [(0, nucleo), (.3, nucleo * .43), (.62, nucleo * .13), (1, 0), (2, 0)])
    sil = Image.fromarray(np.uint8(LOGOS[id] * 255), "L")
    lejos = Image.new("L", (A, A))
    g = sil.resize((round(L * 1.2), round(L * 1.2)), Image.LANCZOS)
    lejos.paste(g, ((A - g.width) // 2, (A - g.height) // 2))
    pegada = Image.new("L", (A, A))
    pegada.paste(sil, ((A - L) // 2, (A - L) // 2))
    return sobre(grad, difuminar(a_float(lejos), 44) * ancha, difuminar(a_float(pegada), 13) * cerca)


# ------------------------------------------------------------------ humo (ruido de la maqueta)

def ruido(semilla, octavas):
    s = semilla & 0xFFFFFFFF

    def rnd():
        nonlocal s
        s = (s * 1664525 + 1013904223) & 0xFFFFFFFF
        return s / 4294967296

    val = np.array([rnd() for _ in range(256)], dtype=np.float32)
    perm = list(range(256))
    for i in range(255, 0, -1):
        j = int(rnd() * (i + 1))
        perm[i], perm[j] = perm[j], perm[i]
    perm = np.array(perm + perm, dtype=np.int64)

    def v(x, y):
        return val[perm[perm[x & 255] + (y & 255)]]

    def base(x, y):
        xi, yi = np.floor(x).astype(np.int64), np.floor(y).astype(np.int64)
        u, w = x - xi, y - yi
        u, w = u * u * (3 - 2 * u), w * w * (3 - 2 * w)
        a, b, c, d = v(xi, yi), v(xi + 1, yi), v(xi, yi + 1), v(xi + 1, yi + 1)
        return a + (b - a) * u + (c - a) * w + (a - b - c + d) * u * w

    def fbm(x, y):
        n, amp, f = 0, .5, 1
        for o in range(octavas):
            n = n + amp * base(x * f + o * 17.3, y * f + o * 9.1)
            amp *= .5
            f *= 2
        return n

    return fbm


def humo(semilla):
    T = 320
    yy, xx = np.mgrid[0:T, 0:T].astype(np.float64)
    dx, dy = xx / T * 2 - 1, yy / T * 2 - 1
    d2 = dx * dx + dy * dy
    n = ruido(semilla, 5)(xx * 3.2 / T, yy * 3.2 / T)
    k = np.clip((n - .3) / .5, 0, 1) ** 1.5 * np.clip(1 - d2, 0, 1) ** 2
    return k.astype(np.float32)


# ------------------------------------------------------------------ fuego nublado de Arch (pintarNublado de la maqueta)
# Capa de celdas de 2 px alrededor del logo: dos texturas de ruido que se repiten cada 256, dos
# envolventes (llamas pegadas al borde y la nube) y dos paletas; el programa solo las combina.

NZ = (-200, -380, 400, 520)  # zona respecto al centro del logo: x, y, w, h
NC = 2
NW, NH = NZ[2] // NC, NZ[3] // NC


def textura(semilla, p0, octavas):
    """Ruido de Perlin que se repite cada 256, ecualizado a 0..255 (mismo azar que la maqueta)."""
    s = semilla & 0xFFFFFFFF

    def rnd():
        nonlocal s
        s = (s * 1664525 + 1013904223) & 0xFFFFFFFF
        return s / 4294967296

    N = 256
    f = np.zeros((N, N))
    p, amp = p0, 1.0
    for _ in range(octavas):
        ang = np.array([rnd() for _ in range(p * p)]) * 2 * np.pi
        gx, gy = np.cos(ang), np.sin(ang)
        c = np.arange(N) / (N / p)
        i = np.floor(c).astype(np.int64)
        d = c - i
        u = d * d * d * (d * (d * 6 - 15) + 10)
        a0, a1 = (i % p) * p, ((i + 1) % p) * p
        b0, b1 = i % p, (i + 1) % p
        j0, j1, dy, v = a0[:, None], a1[:, None], d[:, None], u[:, None]
        i0, i1, dx, w = b0[None, :], b1[None, :], d[None, :], u[None, :]
        n00 = gx[j0 + i0] * dx + gy[j0 + i0] * dy
        n10 = gx[j0 + i1] * (dx - 1) + gy[j0 + i1] * dy
        n01 = gx[j1 + i0] * dx + gy[j1 + i0] * (dy - 1)
        n11 = gx[j1 + i1] * (dx - 1) + gy[j1 + i1] * (dy - 1)
        arriba, abajo = n00 + (n10 - n00) * w, n01 + (n11 - n01) * w
        f += amp * (arriba + (abajo - arriba) * v)
        p, amp = p * 2, amp * .5
    t = np.empty(N * N, np.uint8)
    t[np.argsort(f.ravel(), kind="stable")] = np.arange(N * N) * 256 // (N * N)
    return t.reshape(N, N)


def caja(F, r):
    """Desenfoque de caja de radio r (en x y luego en y), repitiendo el borde."""
    n = 2 * r + 1
    P = np.pad(F, ((0, 0), (r, r)), mode="edge")
    F = sum(P[:, k:k + NW] for k in range(n)) / n
    P = np.pad(F, ((r, r), (0, 0)), mode="edge")
    return sum(P[k:k + NH, :] for k in range(n)) / n


def silueta_en_celdas():
    g = Image.fromarray(np.uint8(np.rint(LOGOS["arch"] * 255)), "L").resize((L // NC, L // NC), Image.BOX)
    m = np.zeros((NH, NW))
    x0, y0 = (-L // 2 - NZ[0]) // NC, (-L // 2 - NZ[1]) // NC
    m[y0:y0 + g.height, x0:x0 + g.width] = a_float(g)[:NH - y0]  # lo de abajo del lienzo del logo queda fuera (vacío)
    return m


SIL = silueta_en_celdas()
# celdas con logo encima en su columna (el hueco de abajo y bajo los pies): ahí no hay fuego
DEBAJO = caja(np.vstack([np.zeros((1, NW)), np.maximum.accumulate(SIL, axis=0)[:-1]]), 3)


def envolvente(largo, cono, radio, dentro, sube=0):
    """La silueta arrastrada hacia arriba y hacia el centro (en cono), suavizada y subida `sube`
    celdas; dentro del logo y debajo de él casi nada (o nada); se apaga en los bordes de la capa."""
    d, xc = np.exp(-NC / largo), NW / 2
    xs = xc + (np.arange(NW) - xc) * (1 + cono)
    x0 = np.floor(xs).astype(np.int64)
    f = xs - x0

    def en(fila, x):
        return np.where((x >= 0) & (x < NW), fila[np.clip(x, 0, NW - 1)], 0)

    S = np.zeros((NH, NW))
    S[NH - 1] = SIL[NH - 1]
    for y in range(NH - 2, -1, -1):
        S[y] = np.maximum(SIL[y], (en(S[y + 1], x0) * (1 - f) + en(S[y + 1], x0 + 1) * f) * d)
    S = caja(caja(S, radio), radio)
    if sube:
        S = np.vstack([S[sube:], np.zeros((sube, NW))])
    yy = np.arange(NH)[:, None]
    S *= (1 - dentro * np.maximum(SIL, DEBAJO)) * np.clip(yy / 30, 0, 1) * np.clip((NH - 1 - yy) / 20, 0, 1)
    return np.floor(np.clip(S, 0, 1) * 255 + .5).astype(np.uint8)


def paleta(paradas):
    """256 colores (filas de r, g, b) interpolando las paradas [(valor, r, g, b)]."""
    v = np.arange(256)
    cols = [np.interp(v, [p[0] for p in paradas], [p[k] for p in paradas]) for k in (1, 2, 3)]
    return np.floor(np.stack(cols, axis=-1) + .5).astype(np.uint8)


NUBLADO = {
    "NUBLADO_RUIDO": textura(7, 4, 5),
    "NUBLADO_GIRO": textura(19, 2, 3),
    "NUBLADO_CERCA": envolvente(38, .004, 2, .85),
    # la nube nunca encima del logo ni a los lados de los pies: empieza donde acaban las llamas
    "NUBLADO_NUBE": envolvente(230, .006, 6, 1, 14),
    "PALETA_FUEGO": paleta([(0, 0, 0, 0), (40, 18, 0, 44), (95, 64, 10, 150), (145, 116, 48, 230), (190, 155, 92, 255), (228, 204, 166, 255), (255, 244, 236, 255)]),
    "PALETA_HUMO": paleta([(0, 0, 0, 0), (90, 24, 4, 56), (170, 54, 16, 118), (255, 88, 36, 176)]),
}


# ------------------------------------------------------------------ textos y panel


def texto(cadena, tam, espacio=0.0, fuente=TEXTO):
    """Máscara de un texto; ancla = centro del texto visible en x y línea base en y."""
    f = ImageFont.truetype(fuente, tam)
    asc, desc = f.getmetrics()
    if espacio:
        avance = sum(f.getlength(c) for c in cadena) + espacio * len(cadena)
    else:
        avance = f.getlength(cadena)
    w, h = ceil(avance) + 6, asc + desc + 6
    im = Image.new("L", (w, h))
    d = ImageDraw.Draw(im)
    if espacio:
        x = 3.0
        for c in cadena:
            d.text((x, 3 + asc), c, font=f, fill=255, anchor="ls")
            x += f.getlength(c) + espacio
    else:
        d.text((3, 3 + asc), cadena, font=f, fill=255, anchor="ls")
    return a_float(im), 3 + (avance - espacio) / 2, 3 + asc


def icono(glifo, tam):
    f = ImageFont.truetype(SIMBOLOS, tam)
    im = Image.new("L", (tam * 2, tam * 2))
    ImageDraw.Draw(im).text((tam, tam), glifo, font=f, fill=255, anchor="mm")
    im = im.crop(im.getbbox())
    return a_float(im), im.width / 2, im.height / 2


def rectangulo_redondo(w, h, r, borde=None):
    S = 4
    im = Image.new("L", (w * S, h * S))
    d = ImageDraw.Draw(im)
    d.rounded_rectangle([0, 0, w * S - 1, h * S - 1], r * S, fill=255)
    if borde:
        d.rounded_rectangle([borde * S, borde * S, (w - borde) * S - 1, (h - borde) * S - 1], (r - borde) * S, fill=0)
    return a_float(im.resize((w, h), Image.BOX))


# ------------------------------------------------------------------ salida

sprites = []  # (nombre, w, h, cx, cy, bytes)


def mascara(nombre, arr, cx=None, cy=None):
    h, w = arr.shape
    sprites.append((nombre, w, h, w / 2 if cx is None else cx, h / 2 if cy is None else cy,
                    np.clip(np.rint(arr * 255), 0, 255).astype(np.uint8).tobytes()))


for id, arr in LOGOS.items():
    mascara(f"LOGO_{id.upper()}", arr)
    mascara(f"AURA_{id.upper()}", aura(id))
for i, s in enumerate([11, 23, 37]):
    # a la mitad: se dibuja a un cuarto de resolución y así no parpadea al girar
    mascara(f"HUMO_{i}", a_float(a_mascara(humo(s)).resize((160, 160), Image.LANCZOS)))
# luz de detrás de la ventana al lanzar Windows: radio 128 (lo que usa Capa::girada)
mascara("LUZ", radial(258, [(0, 1), (.35, .55), (1, 0), (2, 0)]))
for nombre, arr in NUBLADO.items():
    # ya en bytes (las paletas, 256 filas de r, g, b)
    h, w = arr.shape
    sprites.append((nombre, w, h, w / 2, h / 2, arr.tobytes()))

# letra de cada sistema, elegida en maqueta/letras.html (2026-10-27)
NOMBRES = {
    "windows": ("WINDOWS", "selawksb.ttf"),  # Selawik Semibold: la alternativa libre de Microsoft a Segoe UI
    "arch": ("ARCH", "Exo2-SemiBold.ttf"),  # Exo 2 a 600: geométrica técnica
    "kali": ("KALI", "VT323-Regular.ttf"),  # VT323: terminal antigua de fósforo
}
for id, (nombre, fuente) in NOMBRES.items():
    mascara(f"NOMBRE_{id.upper()}", *texto(nombre, 25, 8, str(REC / fuente)))
for n in range(1, 11):
    mascara(f"CUENTA_{n}", *texto(f"Se inicia en {n} s", 20, 2))
mascara("MENU_TITULO", *texto("MÁS OPCIONES", 19, 8))
mascara("MENU_PIE", *texto("Esc  volver     ·     Intro  abrir", 18, 2))
OPCIONES = [("\U000F087E", "Rescate", ""), ("\U000F0279", "rEFInd", ""), ("\U000F061A", "Configuración UEFI", ""),
            ("\U000F0709", "Reiniciar", ""), ("\U000F0425", "Apagar", "")]
for i, (ic, nombre, detalle) in enumerate(OPCIONES):
    mascara(f"OPCION_{i}_ICONO", *icono(ic, 32))
    arr, _, base = texto(nombre, 26)
    mascara(f"OPCION_{i}_NOMBRE", arr, 3, base)  # alineado a la izquierda
    if detalle:
        arr, _, base = texto(detalle, 18)
        mascara(f"OPCION_{i}_DETALLE", arr, 3, base)
for clave, quien in [("WINDOWS", "Windows"), ("ARCH", "Arch"), ("KALI", "Kali"), ("RESCATE", "Limine"), ("REFIND", "rEFInd"), ("BIOS", "la BIOS")]:
    mascara(f"ERROR_{clave}", *texto(f"No se ha podido abrir {quien}: revisa menu.conf", 22, 1))
mascara("PANEL", rectangulo_redondo(692, 562, 18))
mascara("PANEL_BORDE", rectangulo_redondo(692, 562, 18, 1))
mascara("RESALTE", rectangulo_redondo(640, 70, 12))
for d in "0123456789":
    mascara(f"DIGITO_{d}", *texto(d, 20))
for clave, etiqueta in [("FPS", "fps"), ("DIBUJO", "dibujar (ms)"), ("PANTALLA", "pantalla (ms)"), ("TECLADO", "teclado (ms)"),
                       ("VIDEO_WC", "vídeo: write-combining (W lo quita)"), ("VIDEO_UC", "vídeo: sin caché (W lo acelera)"),
                       ("VIDEO_FALLO", "vídeo: no se pudo acelerar")]:
    arr, _, base = texto(etiqueta, 20)
    mascara(f"MEDIDA_{clave}", arr, 3, base)

blob = bytearray()
lineas = ["// Generado por herramientas/sprites.py: no editar a mano.", "use crate::grafico::Sprite;", ""]
for nombre, w, h, cx, cy, datos in sprites:
    while len(blob) % 4:
        blob.append(0)
    lineas.append(f"pub const {nombre}: Sprite = Sprite {{ inicio: {len(blob)}, w: {w}, h: {h}, cx: {cx:.2f}, cy: {cy:.2f} }};")
    blob += datos
lineas += ["", f"pub const L: f32 = {L}.0;",
           "/// Cuadros del logo de Windows dentro de su sprite: x, y, w, h",
           "pub const CUADROS: [[f32; 4]; 4] = [" + ", ".join("[" + ", ".join(f"{v:.1f}" for v in c) + "]" for c in CUADROS) + "];",
           "/// Zona del fuego de Arch respecto al centro del logo (x, y, w, h); celdas de NUBLADO_NUBE",
           "pub const NUBLADO_ZONA: [f32; 4] = [" + ", ".join(f"{v:.1f}" for v in NZ) + "];",
           "pub static DATOS: &[u8] = include_bytes!(\"recursos.bin\");", ""]
salida = RAIZ / "app" / "src"
(salida / "recursos.bin").write_bytes(bytes(blob))
(salida / "recursos.rs").write_text("\n".join(lineas), encoding="utf-8", newline="\n")
print(f"{len(sprites)} sprites, {len(blob) / 1e6:.2f} MB")

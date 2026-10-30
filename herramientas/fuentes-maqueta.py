"""Genera maqueta/fuentes.js: las fuentes de la maqueta en base64 (Chrome no carga
fuentes desde file://). De la Nerd Font solo se quedan los glifos que se usan."""
import base64
from pathlib import Path

from fontTools import subset

RAIZ = Path(__file__).resolve().parent.parent
RECURSOS = RAIZ / "recursos"

# Arch, Kali y los iconos del menú de Esc (Limine, rEFInd, UEFI, reiniciar, apagar)
GLIFOS = [0xF303, 0xF327, 0xF087E, 0xF0279, 0xF061A, 0xF0709, 0xF0425]


def simbolos() -> bytes:
    opciones = subset.Options()
    opciones.layout_features = []
    opciones.name_IDs = ["*"]
    fuente = subset.load_font(str(RECURSOS / "SymbolsNerdFont-Regular.ttf"), opciones)
    sub = subset.Subsetter(opciones)
    sub.populate(unicodes=GLIFOS)
    sub.subset(fuente)
    salida = RAIZ / "maqueta" / "_simbolos.ttf"
    subset.save_font(fuente, str(salida), opciones)
    datos = salida.read_bytes()
    salida.unlink()
    return datos


def recorte(fichero: str, letras: str) -> bytes:
    """Solo las letras de un nombre (la letra de cada sistema)."""
    opciones = subset.Options()
    opciones.name_IDs = ["*"]
    fuente = subset.load_font(str(RECURSOS / fichero), opciones)
    sub = subset.Subsetter(opciones)
    sub.populate(text=letras)
    sub.subset(fuente)
    salida = RAIZ / "maqueta" / "_recorte.ttf"
    subset.save_font(fuente, str(salida), opciones)
    datos = salida.read_bytes()
    salida.unlink()
    return datos


def uri(tipo: str, datos: bytes) -> str:
    return f"data:font/{tipo};base64," + base64.b64encode(datos).decode()


fuentes = {
    "simbolos": uri("ttf", simbolos()),
    "texto500": uri("woff2", (RECURSOS / "alegreya-sans-normal-500-latin.woff2").read_bytes()),
    "texto700": uri("woff2", (RECURSOS / "alegreya-sans-normal-700-latin.woff2").read_bytes()),
    # la letra del nombre de cada sistema (como en herramientas/sprites.py)
    "windows": uri("ttf", recorte("selawksb.ttf", "WINDOWS")),
    "arch": uri("ttf", recorte("Exo2-SemiBold.ttf", "ARCH")),
    "kali": uri("ttf", recorte("VT323-Regular.ttf", "KALI")),
}
js = "// Generado por herramientas/fuentes-maqueta.py: no editar a mano.\nwindow.FUENTES = {\n"
js += "".join(f"  {k}: '{v}',\n" for k, v in fuentes.items()) + "};\n"
(RAIZ / "maqueta" / "fuentes.js").write_text(js, encoding="utf-8", newline="\n")
print({k: len(v) for k, v in fuentes.items()})

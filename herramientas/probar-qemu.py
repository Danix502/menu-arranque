"""Prueba el menú en QEMU (OVMF) sin tocar el portátil: monta una partición EFI de prueba,
arranca la máquina sin ventana y sigue un guion de esperas, teclas y capturas por QMP.

  python herramientas/probar-qemu.py <carpeta-de-capturas> [guion]

Guion: pasos separados por comas; «2.5» espera, «izq»/«der»/«arr»/«abj»/«intro»/«esc»/«f»
pulsa, «foto:nombre» guarda una captura. La salida del puerto serie (lo que escribe el
destino de prueba) queda en serie.log, en la carpeta de capturas. QEMU_ACCEL=tcg fuerza la CPU
emulada (WHPX no deja tocar los MTRR como un equipo real)."""
import json
import os
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path

from PIL import Image

RAIZ = Path(__file__).resolve().parent.parent
QEMU = Path.home() / "herramientas" / "qemu"
EFI = RAIZ / "app" / "target" / "x86_64-unknown-uefi" / "release"
TECLAS = {"izq": "left", "der": "right", "arr": "up", "abj": "down", "intro": "ret", "esc": "esc", "f": "f"}
CONF_PRUEBA = """# menu.conf de prueba (QEMU): todo apunta al destino de prueba
espera = 10
windows = \\EFI\\prueba\\prueba.efi
arch = \\EFI\\prueba\\prueba.efi
arch_opciones = initrd=\\arch\\initramfs-linux.img root=UUID=prueba rw quiet
kali = \\EFI\\no-existe\\vmlinuz
rescate = \\EFI\\prueba\\prueba.efi
"""


class Qmp:
    def __init__(self, puerto):
        for _ in range(100):
            try:
                self.s = socket.create_connection(("127.0.0.1", puerto), timeout=60)
                break
            except OSError:
                time.sleep(.2)
        else:
            raise SystemExit("QEMU no abre QMP")
        self.f = self.s.makefile("r", encoding="utf-8")
        self.f.readline()  # saludo
        self.orden("qmp_capabilities")

    def orden(self, nombre, **args):
        self.s.sendall((json.dumps({"execute": nombre, "arguments": args} if args else {"execute": nombre}) + "\n").encode())
        while True:
            r = json.loads(self.f.readline())
            if "return" in r or "error" in r:
                return r


def main():
    salida = Path(sys.argv[1]).resolve()
    guion = sys.argv[2] if len(sys.argv) > 2 else "4,foto:inicio"
    salida.mkdir(parents=True, exist_ok=True)
    esp = salida / "esp"
    shutil.rmtree(esp, ignore_errors=True)
    (esp / "EFI" / "BOOT").mkdir(parents=True)
    (esp / "EFI" / "menu-arranque").mkdir(parents=True)
    (esp / "EFI" / "prueba").mkdir(parents=True)
    shutil.copy(EFI / "menu-arranque.efi", esp / "EFI" / "BOOT" / "BOOTX64.EFI")
    shutil.copy(EFI / "prueba.efi", esp / "EFI" / "prueba" / "prueba.efi")
    (esp / "EFI" / "menu-arranque" / "menu.conf").write_text(CONF_PRUEBA, encoding="utf-8", newline="\n")
    vars_ = salida / "vars.fd"
    shutil.copy(QEMU / "share" / "edk2-i386-vars.fd", vars_)
    serie = salida / "serie.log"
    orden = [str(QEMU / "qemu-system-x86_64.exe"), "-machine", "q35", "-m", "1024", *(["-accel", os.environ["QEMU_ACCEL"]] if "QEMU_ACCEL" in os.environ else ["-accel", "whpx", "-accel", "tcg"]),
             "-drive", f"if=pflash,format=raw,readonly=on,file={QEMU / 'share' / 'edk2-x86_64-code.fd'}",
             "-drive", f"if=pflash,format=raw,file={vars_}",
             "-drive", f"format=raw,file=fat:rw:{esp}",
             "-vga", "std", "-display", "none", "-serial", f"file:{serie}",
             "-qmp", "tcp:127.0.0.1:4455,server=on,wait=off"]
    qemu = subprocess.Popen(orden, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    try:
        q = Qmp(4455)
        t0 = time.time()
        for paso in guion.split(","):
            paso = paso.strip()
            if paso.startswith("foto:"):
                ppm = salida / (paso[5:] + ".ppm")
                q.orden("screendump", filename=str(ppm))
                time.sleep(.3)
                Image.open(ppm).save(ppm.with_suffix(".png"))
                ppm.unlink()
                print(f"{time.time() - t0:6.1f}s captura {paso[5:]} {Image.open(ppm.with_suffix('.png')).size}")
            elif paso in TECLAS:
                q.orden("send-key", keys=[{"type": "qcode", "data": TECLAS[paso]}])
            else:
                time.sleep(float(paso))
    finally:
        qemu.kill()
        print(qemu.stdout.read().decode(errors="replace")[-800:])
    if serie.exists():
        print("serie:", "".join(l for l in serie.read_text(errors="replace").splitlines(True) if "PRUEBA" in l) or "(nada del destino de prueba)")


main()

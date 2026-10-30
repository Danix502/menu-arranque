[Español](README.md) · **English**

# Boot menu

An animated boot menu for picking an operating system at power-on, written from scratch as a **UEFI program in Rust**. It replaces the GRUB or rEFInd screen: Windows in the middle, Kali on the left and Arch on the right, each with its own animation.

![Arch selected: the logo burns with purple fire](docs/arch.png)

| Kali selected | More options (Esc) |
|---|---|
| ![Kali with a glitch effect](docs/kali.png) | ![More options menu](docs/esc.png) |

The on-screen text and the code are in Spanish (*Rescate* = Rescue, *Reiniciar* = Restart, *Apagar* = Shut down).

## Features
- **← →** to choose, **Enter** to boot. Windows is the default and boots by itself after 10 s; any key stops the countdown.
- Each system has its own effect when selected (Kali glitches, Windows gets a blue aura, Arch catches fire) and its own launch animation (Kali switches off like an old TV, Windows opens its panes towards the light, Arch burns like a sheet of paper).
- **Esc** opens more options: **Rescue** (another boot manager, such as Limine), **rEFInd**, **UEFI settings** (enters the firmware setup), **Restart** and **Shut down**.
- **F** shows fps and frame time.
- If something fails to boot, it shows a message and goes back to the menu instead of hanging.

## How it works
There is no operating system underneath: the UEFI firmware loads the `.efi` and the program draws straight to the screen (GOP, 1920×1080).
- **Everything expensive happens at build time.** `herramientas/sprites.py` (Pillow + numpy) renders logos, text, gradients, smoke and the fire textures, and bakes them into the binary as masks. Each frame only blits sprites with scale, rotation and alpha.
- **Integers only in pixel loops.** The `x86_64-unknown-uefi` target has no SSE, so floating point is done in software (about 10× slower). Drawing uses fixed point.
- **Fast video memory.** Some firmwares map video memory as uncached (25 fps on the original machine). On startup the menu sets a free MTRR to *write-combining* (over 180 fps; capped at 60) and releases it before starting the chosen system.
- **Linux is booted directly** through the kernel's EFI stub, with its command line and `initrd=`; Windows, rEFInd and Limine are regular `.efi` files.

## Layout
| Folder | Contents |
|---|---|
| `app/` | The UEFI program (`main.rs` loop and video, `escena.rs` the scene, `grafico.rs` drawing, `arranque.rs` `menu.conf` and booting, `video.rs` the MTRR) |
| `herramientas/` | `sprites.py` generates the assets; `probar-qemu.py` runs it in a VM and takes screenshots |
| `maqueta/` | The design in HTML/JS: open `maqueta/index.html` in a browser to see it without building anything |
| `banco/` | The same drawing code built for Windows, to measure frame time |
| `recursos/` | Fonts (OFL) and Symbols Nerd Font (MIT), with their licenses |

## Building
You need Rust 1.85 or newer and Python 3 with Pillow and numpy.

```sh
rustup target add x86_64-unknown-uefi
pip install pillow numpy
python herramientas/sprites.py      # generates app/src/recursos.bin and recursos.rs
cd app && cargo build --release
```

Output: `app/target/x86_64-unknown-uefi/release/menu-arranque.efi`.

## Trying it without touching your machine
`herramientas/probar-qemu.py` boots the menu in QEMU with OVMF and follows a script of keys and screenshots (`izq`/`der` = left/right, `intro` = Enter, `foto:name` = screenshot, numbers = seconds to wait):

```sh
python herramientas/probar-qemu.py shots "3,foto:start,der,2.5,foto:arch,esc,1,foto:menu"
```

It looks for QEMU in `~/herramientas/qemu`: change the `QEMU` path in the script if yours is elsewhere.

## Installing
> ⚠️ You are about to change how your machine boots. Keep rEFInd, GRUB or your current boot manager as the second entry so you can go back, and if you use BitLocker, suspend it first (`Suspend-BitLocker -MountPoint C: -RebootCount 2`) or it will ask for the recovery key. The `.efi` is not signed: it won't boot with **Secure Boot** enabled.

1. Copy the program and its config to the EFI partition (on Linux usually mounted at `/boot` or `/efi`):
   ```sh
   sudo mkdir -p /boot/EFI/menu-arranque
   sudo cp app/target/x86_64-unknown-uefi/release/menu-arranque.efi /boot/EFI/menu-arranque/
   sudo cp menu.conf.ejemplo /boot/EFI/menu-arranque/menu.conf
   ```
2. Edit `menu.conf` (below) with the paths to your systems.
3. Create the UEFI entry and put it first (use the disk and partition number of your EFI partition):
   ```sh
   sudo efibootmgr --create --disk /dev/nvme0n1 --part 1 --label "Boot menu" \
     --loader '\EFI\menu-arranque\menu-arranque.efi'
   sudo efibootmgr            # check the new entry's number and the boot order
   sudo efibootmgr -o XXXX,YYYY,...   # new entry first, your previous manager after it
   ```

### menu.conf
Always at `\EFI\menu-arranque\menu.conf`. Format `key = value`; `#` starts a comment. Paths are inside the EFI partition, with `\`.

| Key | Meaning | Default |
|---|---|---|
| `espera` | Seconds before booting Windows (1-60) | `10` |
| `windows` | Windows boot manager | `\EFI\Microsoft\Boot\bootmgfw.efi` |
| `arch`, `arch_opciones` | Arch kernel and its command line, with `initrd=` | — |
| `kali`, `kali_opciones` | Same for Kali | — |
| `rescate` | What «Rescue» opens | `\EFI\limine\BOOTX64.EFI` |
| `refind` | What «rEFInd» opens | `\EFI\refind\refind_x64.efi` |

```ini
espera = 10
windows = \EFI\Microsoft\Boot\bootmgfw.efi
arch = \EFI\arch\vmlinuz-linux
arch_opciones = initrd=\EFI\arch\initramfs-linux.img root=UUID=xxxx rw quiet
kali = \kali\vmlinuz
kali_opciones = initrd=\kali\initrd.img root=UUID=yyyy rw
```

The kernel and initrd **must live on the EFI partition** (the firmware can't read ext4 or btrfs). If you already use rEFInd or systemd-boot, copy the paths and options from its config (`refind.conf` or `loader/entries/`). If your distro keeps the kernel on another partition, a hook that copies it to the EFI partition after every update solves it (on Debian and Kali: `/etc/kernel/postinst.d/` and `/etc/initramfs/post-update.d/`).

## Customising
It's built for Windows + Arch + Kali at 1920×1080. For other systems or colours: logos and text come from `herramientas/sprites.py` (Nerd Font logos), effects from `app/src/escena.rs`, and it's worth trying the change in the mockup first (`maqueta/menu.js`).

## License
Code under [MIT](LICENSE). The fonts in `recursos/` keep their own licenses (OFL and MIT), included next to each one. Logos are trademarks of their owners.

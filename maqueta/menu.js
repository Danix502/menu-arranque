'use strict';
// Maqueta del menú de arranque del portátil, a 1920×1080 como la pantalla real.
// Pensada para pasarla tal cual a la app UEFI: lo caro (degradados, desenfoques, glifos, ruido)
// se hace una vez al cargar, como lo hará Pillow al compilar, y en cada fotograma solo se
// dibujan esos sprites con escala, giro y alfa (mezcla normal o aditiva), franjas de un sprite
// (glitch) y textos fijos.
(async () => {
  const W = 1920, H = 1080, Y = 455, SEP = 470;
  const lienzo = document.getElementById('pantalla');
  const ctx = lienzo.getContext('2d');

  // la letra del nombre de cada sistema: Selawik Semibold, Exo 2 a 600 y VT323 (elegidas en letras.html)
  for (const [familia, uri, weight] of [['Simbolos', FUENTES.simbolos, '400'], ['Texto', FUENTES.texto500, '500'], ['Texto', FUENTES.texto700, '700'],
    ['Nombre-windows', FUENTES.windows, '400'], ['Nombre-arch', FUENTES.arch, '400'], ['Nombre-kali', FUENTES.kali, '400']]) {
    document.fonts.add(await new FontFace(familia, `url(${uri})`, { weight }).load());
  }

  // ================================================================ datos
  const SISTEMAS = {
    // brillo del aura de Windows más bajo y sin la capa pegada al logo: si no, se funde con su azul
    windows: { nombre: 'WINDOWS', logo: [14, 128, 224], aura: [30, 110, 255], brillo: [.2, .6, .12], caja: [184, 184], destino: '\\EFI\\Microsoft\\Boot\\bootmgfw.efi' },
    arch: { nombre: 'ARCH', logo: [155, 92, 255], aura: [139, 61, 255], glifo: '', ancho: 1944 / 2048, caja: [206, 196], destino: 'el kernel de Arch, con su initramfs' },
    kali: { nombre: 'KALI', logo: [192, 24, 47], aura: [226, 32, 62], glifo: '', caja: [262, 196], destino: '/kali/vmlinuz, con su initrd' },
  };
  const IDS = Object.keys(SISTEMAS);
  const NOMBRE = { windows: 'Windows', arch: 'Arch', kali: 'Kali' };
  const OPCIONES = [
    { icono: 0xF087E, nombre: 'Rescate', detalle: '', aviso: 'Aquí se abriría Limine' },
    { icono: 0xF0279, nombre: 'rEFInd', detalle: '', aviso: 'Aquí se abriría rEFInd' },
    { icono: 0xF061A, nombre: 'Configuración UEFI', detalle: '', aviso: 'Aquí se reiniciaría a la BIOS' },
    { icono: 0xF0709, nombre: 'Reiniciar', detalle: '', aviso: 'Aquí se reiniciaría' },
    { icono: 0xF0425, nombre: 'Apagar', detalle: '', aviso: 'Aquí se apagaría' },
  ];
  const ORDENES = { kwa: ['kali', 'windows', 'arch'], awk: ['arch', 'windows', 'kali'] };
  const cfg = { fondo: 'humo', windows: 'cuadros', arch: 'nublado', kali: 'glitch', orden: 'kwa', pista: 'no', lanzar: 'propio' };

  // Sombras del fondo: rojo sangre y púrpura de la paleta
  const SOMBRA = { rojo: [150, 8, 30], morado: [92, 10, 196] };

  // Chispas (ráfaga al arrancar, órbita de Arch, brasas de Kali): tres temperaturas por paleta
  const BRASAS = {
    windows: [['rgba(235,245,255,1)', 'rgba(130,190,255,.9)', 'rgba(40,120,255,.35)'], ['rgba(180,215,255,1)', 'rgba(70,150,255,.85)', 'rgba(20,80,230,.3)'], ['rgba(110,160,255,.9)', 'rgba(30,80,210,.7)', 'rgba(10,30,130,.25)']],
    arch: [['rgba(245,235,255,1)', 'rgba(200,160,255,.9)', 'rgba(150,80,255,.35)'], ['rgba(215,180,255,1)', 'rgba(155,92,255,.85)', 'rgba(107,0,214,.3)'], ['rgba(170,110,255,.9)', 'rgba(107,0,214,.7)', 'rgba(50,0,110,.25)']],
    kali: [['rgba(255,230,230,1)', 'rgba(255,130,140,.9)', 'rgba(230,30,60,.35)'], ['rgba(255,175,180,1)', 'rgba(232,48,72,.85)', 'rgba(192,24,47,.3)'], ['rgba(235,90,100,.9)', 'rgba(160,12,32,.7)', 'rgba(80,0,12,.25)']],
  };
  // Llama de Arch: base, centro y punta de cada lengua, de más caliente a más fría
  const LLAMA = [
    ['rgba(255,248,255,1)', 'rgba(220,190,255,.95)', 'rgba(170,110,255,0)'],
    ['rgba(225,195,255,.95)', 'rgba(155,92,255,.85)', 'rgba(107,0,214,0)'],
    ['rgba(165,105,255,.85)', 'rgba(107,0,214,.65)', 'rgba(50,0,110,0)'],
  ];

  // ================================================================ utilidades
  const rgba = (c, a) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
  const aclarar = (c, k) => c.map((v) => Math.round(v + (255 - v) * k));
  const lim = (v, a = 0, b = 1) => Math.max(a, Math.min(b, v));
  const suave = (v) => { v = lim(v); return v * v * (3 - 2 * v); };
  const salida = (v) => 1 - Math.pow(1 - lim(v), 3);
  const aprox = (v, obj, vel, dt) => v + (obj - v) * (1 - Math.exp(-vel * dt));
  const azar = (a, b) => a + Math.random() * (b - a);
  const nuevo = (w, h = w) => Object.assign(document.createElement('canvas'), { width: w, height: h });
  const posX = (id) => W / 2 + (ORDENES[cfg.orden].indexOf(id) - 1) * SEP;

  // Ruido de valor con semilla (para el humo): fbm(x, y) da ~0..1
  function ruido(semilla, octavas) {
    let s = semilla >>> 0;
    const rnd = () => (s = (s * 1664525 + 1013904223) >>> 0) / 4294967296;
    const val = Float32Array.from({ length: 256 }, rnd), perm = new Uint8Array(512);
    for (let i = 0; i < 256; i++) perm[i] = i;
    for (let i = 255; i > 0; i--) { const j = (rnd() * (i + 1)) | 0; [perm[i], perm[j]] = [perm[j], perm[i]]; }
    for (let i = 0; i < 256; i++) perm[i + 256] = perm[i];
    const v = (x, y) => val[perm[perm[x & 255] + (y & 255)]];
    const base = (x, y) => {
      const xi = Math.floor(x), yi = Math.floor(y), u = suave(x - xi), w = suave(y - yi);
      const a = v(xi, yi), b = v(xi + 1, yi), c = v(xi, yi + 1), d = v(xi + 1, yi + 1);
      return a + (b - a) * u + (c - a) * w + (a - b - c + d) * u * w;
    };
    return (x, y) => {
      let n = 0, amp = .5, f = 1;
      for (let o = 0; o < octavas; o++) { n += amp * base(x * f + o * 17.3, y * f + o * 9.1); amp *= .5; f *= 2; }
      return n;
    };
  }

  // ================================================================ sprites (en la app: Pillow al compilar)
  const L = 320, A = 760;

  function hacerLogo(id, color) {
    const s = SISTEMAS[id], c = nuevo(L), g = c.getContext('2d');
    g.fillStyle = color;
    if (id === 'windows') {
      const lado = s.caja[0], hueco = 9, cuadro = (lado - hueco) / 2, x0 = (L - lado) / 2;
      s.cuadros = [[0, 0], [1, 0], [1, 1], [0, 1]].map(([i, j]) => [x0 + i * (cuadro + hueco), x0 + j * (cuadro + hueco), cuadro, cuadro]);
      for (const r of s.cuadros) g.fillRect(...r);
    } else {
      g.font = '400px Simbolos';
      const m = g.measureText(s.glifo);
      const w = (m.actualBoundingBoxLeft + m.actualBoundingBoxRight) * (s.ancho ?? 1), h = m.actualBoundingBoxAscent + m.actualBoundingBoxDescent;
      const k = Math.min(s.caja[0] / w, s.caja[1] / h);
      g.font = `${400 * k}px Simbolos`;
      g.save(); g.beginPath(); g.rect(0, 0, (L + w * k) / 2 + 1, L); g.clip(); // fuera queda el ™ de Arch
      g.fillText(s.glifo, L / 2 + (m.actualBoundingBoxLeft - w / 2) * k, L / 2 + (m.actualBoundingBoxAscent - m.actualBoundingBoxDescent) * k / 2);
      g.restore();
    }
    return c;
  }

  // Halo redondo + silueta muy difuminada + silueta poco difuminada (la que pega al borde del logo)
  function hacerAura(id) {
    const s = SISTEMAS[id], [nucleo, ancha, cerca] = s.brillo ?? [.30, .95, .8], c = nuevo(A), g = c.getContext('2d');
    const r = g.createRadialGradient(A / 2, A / 2, 0, A / 2, A / 2, A / 2);
    r.addColorStop(0, rgba(s.aura, nucleo)); r.addColorStop(.3, rgba(s.aura, nucleo * .43)); r.addColorStop(.62, rgba(s.aura, nucleo * .13)); r.addColorStop(1, rgba(s.aura, 0));
    g.fillStyle = r; g.fillRect(0, 0, A, A);
    const silueta = hacerLogo(id, rgba(s.aura, 1));
    g.filter = 'blur(44px)'; g.globalAlpha = ancha; g.drawImage(silueta, A / 2 - L * .6, A / 2 - L * .6, L * 1.2, L * 1.2);
    g.filter = 'blur(13px)'; g.globalAlpha = cerca; g.drawImage(silueta, (A - L) / 2, (A - L) / 2);
    return c;
  }

  function hacerBrasa([a, b, c]) {
    const s = nuevo(32), g = s.getContext('2d'), r = g.createRadialGradient(16, 16, 0, 16, 16, 16);
    r.addColorStop(0, a); r.addColorStop(.22, b); r.addColorStop(.5, c); r.addColorStop(1, 'rgba(0,0,0,0)');
    g.fillStyle = r; g.fillRect(0, 0, 32, 32);
    return s;
  }

  // Lengua de fuego: ancha y caliente abajo, afilada y fría arriba
  function hacerLengua([base, centro, punta]) {
    const c = nuevo(32, 80), g = c.getContext('2d');
    const gr = g.createLinearGradient(0, 74, 0, 4);
    gr.addColorStop(0, base); gr.addColorStop(.4, centro); gr.addColorStop(1, punta);
    g.filter = 'blur(2.5px)'; g.fillStyle = gr;
    g.beginPath();
    g.moveTo(16, 4);
    g.bezierCurveTo(20, 26, 27, 42, 26, 58);
    g.bezierCurveTo(25, 72, 7, 72, 6, 58);
    g.bezierCurveTo(5, 42, 12, 26, 16, 4);
    g.fill();
    return c;
  }

  // Bocanada de humo: ruido recortado en círculo, teñida de rojo o morado
  function hacerHumo(semilla) {
    const T = 320, c = nuevo(T), g = c.getContext('2d'), img = g.createImageData(T, T), n = ruido(semilla, 5);
    for (let y = 0; y < T; y++) for (let x = 0; x < T; x++) {
      const dx = x / T * 2 - 1, dy = y / T * 2 - 1, d2 = dx * dx + dy * dy;
      if (d2 >= 1) continue;
      const k = Math.pow(lim((n(x * 3.2 / T, y * 3.2 / T) - .3) / .5), 1.5) * Math.pow(1 - d2, 2);
      img.data[(y * T + x) * 4 + 3] = 255 * k;
    }
    g.putImageData(img, 0, 0);
    return Object.fromEntries(Object.entries(SOMBRA).map(([nombre, col]) => {
      const t = nuevo(T), h = t.getContext('2d');
      h.drawImage(c, 0, 0); h.globalCompositeOperation = 'source-in'; h.fillStyle = rgba(col, 1); h.fillRect(0, 0, T, T);
      return [nombre, t];
    }));
  }

  // Sombra que entra por un lado: rojo por la izquierda y abajo, morado por la derecha y arriba.
  // A un cuarto de resolución: es tan suave que se agranda sin que se note.
  function hacerBorde(col, izquierda, semilla) {
    const w = 480, h = 270, c = nuevo(w, h), g = c.getContext('2d'), img = g.createImageData(w, h), n = ruido(semilla, 4);
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
      const u = izquierda ? x / w : 1 - x / w, v = izquierda ? 1 - y / h : y / h;
      const base = Math.exp(-u * 7) * .95 + Math.exp(-v * 7.5) * .6 * Math.pow(1 - u, .8);
      const k = lim(base * (.25 + 1.05 * n(x / 55, y / 55)));
      const i = (y * w + x) * 4;
      img.data[i] = col[0]; img.data[i + 1] = col[1]; img.data[i + 2] = col[2]; img.data[i + 3] = 255 * k;
    }
    g.putImageData(img, 0, 0);
    return c;
  }

  function hacerRayos(color) {
    const R = 1000, c = nuevo(R), g = c.getContext('2d');
    g.translate(R / 2, R / 2);
    for (let i = 0; i < 14; i++) {
      const a = i / 14 * Math.PI * 2, ancho = i % 2 ? .045 : .085;
      const gr = g.createRadialGradient(0, 0, 70, 0, 0, R / 2);
      gr.addColorStop(0, rgba(color, .6)); gr.addColorStop(.45, rgba(color, .15)); gr.addColorStop(1, rgba(color, 0));
      g.fillStyle = gr; g.beginPath(); g.moveTo(0, 0); g.arc(0, 0, R / 2, a - ancho, a + ancho); g.closePath(); g.fill();
    }
    const d = nuevo(R), h = d.getContext('2d');
    h.filter = 'blur(10px)'; h.drawImage(c, 0, 0);
    return d;
  }

  function hacerAnillo(color) {
    const R = 640, c = nuevo(R), g = c.getContext('2d');
    g.filter = 'blur(6px)'; g.strokeStyle = rgba(color, .9); g.lineWidth = 7;
    g.beginPath(); g.arc(R / 2, R / 2, 300, 0, Math.PI * 2); g.stroke();
    g.filter = 'none'; g.strokeStyle = rgba(aclarar(color, .45), .9); g.lineWidth = 2;
    g.beginPath(); g.arc(R / 2, R / 2, 300, 0, Math.PI * 2); g.stroke();
    return c;
  }

  // Franja diagonal del destello de Windows (se recorta con la silueta en cada fotograma)
  function hacerBrillo(color) {
    const B = 600, c = nuevo(B), g = c.getContext('2d');
    g.translate(B / 2, B / 2); g.rotate(-.42);
    const gr = g.createLinearGradient(-70, 0, 70, 0);
    gr.addColorStop(0, rgba(color, 0)); gr.addColorStop(.5, rgba(color, .9)); gr.addColorStop(1, rgba(color, 0));
    g.fillStyle = gr; g.fillRect(-70, -B, 140, 2 * B);
    return c;
  }

  // Línea que barre el dragón de Kali de arriba abajo
  function hacerBarrido(color) {
    const c = nuevo(L, 36), g = c.getContext('2d'), gr = g.createLinearGradient(0, 0, 0, 36);
    gr.addColorStop(0, rgba(color, 0)); gr.addColorStop(.5, rgba(color, .95)); gr.addColorStop(1, rgba(color, 0));
    g.fillStyle = gr; g.fillRect(0, 0, L, 36);
    return c;
  }

  function hacerLineas(logo) {
    const c = nuevo(L), g = c.getContext('2d');
    g.fillStyle = 'rgba(0,0,0,.55)';
    for (let y = 0; y < L; y += 4) g.fillRect(0, y, L, 1.6);
    g.globalCompositeOperation = 'destination-in'; g.drawImage(logo, 0, 0);
    return c;
  }

  function hacerPista() {
    const c = nuevo(W, 64), g = c.getContext('2d');
    const grupos = [[['izq', 'der'], 'elegir'], [['Intro'], 'arrancar'], [['Esc'], 'más opciones']];
    g.font = '500 21px Texto'; g.textBaseline = 'middle';
    const tecla = (k) => (k === 'izq' || k === 'der' ? 36 : g.measureText(k).width + 24);
    const anchos = grupos.map(([teclas, texto]) => teclas.reduce((s, k) => s + tecla(k) + 8, 0) + 6 + g.measureText(texto).width);
    const hueco = 64;
    let x = W / 2 - (anchos.reduce((a, b) => a + b, 0) + hueco * (grupos.length - 1)) / 2;
    const y = 32;
    g.strokeStyle = 'rgba(255,255,255,.85)'; g.fillStyle = 'rgba(255,255,255,.95)'; g.lineWidth = 1.5;
    for (const [teclas, texto] of grupos) {
      for (const k of teclas) {
        const w = tecla(k);
        g.beginPath(); g.roundRect(x + .5, y - 17.5, w, 35, 7); g.stroke();
        if (k === 'izq' || k === 'der') {
          const cx = x + w / 2, d = k === 'izq' ? -1 : 1;
          g.beginPath(); g.moveTo(cx + 6 * d, y); g.lineTo(cx - 4 * d, y - 7); g.lineTo(cx - 4 * d, y + 7); g.closePath(); g.fill();
        } else { g.textAlign = 'center'; g.fillText(k, x + w / 2, y + 1); }
        x += w + 8;
      }
      g.textAlign = 'left'; x += 6;
      g.fillText(texto, x, y + 1);
      x += g.measureText(texto).width + hueco;
    }
    return c;
  }

  // Puntos del borde de arriba de una silueta: de ahí salen la llama de Arch y las brasas de Kali
  function bordes(spr) {
    const d = spr.getContext('2d').getImageData(0, 0, L, L).data, pts = [];
    for (let y = 4; y < L; y += 2) for (let x = 0; x < L; x += 2) {
      if (d[(y * L + x) * 4 + 3] > 140 && d[((y - 4) * L + x) * 4 + 3] < 40) pts.push([x - L / 2, y - L / 2]);
    }
    return pts;
  }

  const SPR = { logo: {}, aura: {}, anillo: {}, brasa: {} };
  for (const id of IDS) {
    const s = SISTEMAS[id];
    SPR.logo[id] = hacerLogo(id, rgba(s.logo, 1));
    SPR.aura[id] = hacerAura(id);
    SPR.anillo[id] = hacerAnillo(s.aura);
  }
  for (const [k, temps] of Object.entries(BRASAS)) SPR.brasa[k] = temps.map(hacerBrasa);
  SPR.lengua = LLAMA.map(hacerLengua);
  SPR.humo = [11, 23, 37].map(hacerHumo);
  SPR.borde = { rojo: hacerBorde(SOMBRA.rojo, true, 5), morado: hacerBorde(SOMBRA.morado, false, 9) };
  SPR.rayos = hacerRayos(aclarar(SISTEMAS.windows.aura, .35));
  SPR.brillo = hacerBrillo([215, 236, 255]);
  SPR.cuadros = SISTEMAS.windows.cuadros.map((r) => {
    const c = nuevo(L), g = c.getContext('2d');
    g.fillStyle = rgba(aclarar(SISTEMAS.windows.logo, .6), 1); g.fillRect(...r);
    return c;
  });
  SPR.lineas = hacerLineas(SPR.logo.kali);
  SPR.barrido = hacerBarrido([255, 120, 135]);
  SPR.fantasma = [hacerLogo('kali', 'rgb(255,77,104)'), hacerLogo('kali', 'rgb(122,28,255)')];
  SPR.pista = hacerPista();
  const BORDES = { arch: bordes(SPR.logo.arch), kali: bordes(SPR.logo.kali) };
  const tmp = nuevo(L), tg = tmp.getContext('2d');

  // ================================================================ fuego nublado de Arch (elegido el 2026-10-26 en fuego.html)
  // Una capa de celdas de 2 px alrededor del logo: llamas cortas pegadas al borde y una nube que
  // sube despacio y se enrosca. En cada fotograma solo hay búsquedas en texturas de ruido, cuentas
  // con enteros y una paleta de 256 colores; las texturas y las envolventes se hacen al cargar
  // (en la app, con Pillow al compilar). La capa se agranda ×2 con suavizado y se suma.
  const NZ = { x: -200, y: -380, w: 400, h: 520, c: 2 };
  const NW = NZ.w / NZ.c, NH = NZ.h / NZ.c;
  // Silueta del logo en celdas (0..1)
  const SILUETA = (() => {
    const c = nuevo(NW, NH), g = c.getContext('2d');
    g.drawImage(SPR.logo.arch, (-L / 2 - NZ.x) / NZ.c, (-L / 2 - NZ.y) / NZ.c, L / NZ.c, L / NZ.c);
    const d = g.getImageData(0, 0, NW, NH).data;
    return Float32Array.from({ length: NW * NH }, (_, i) => d[i * 4 + 3] / 255);
  })();
  function desenfocar(F, r) {
    const aux = new Float32Array(F.length), n = 2 * r + 1;
    for (let y = 0; y < NH; y++) for (let x = 0; x < NW; x++) {
      let s = 0;
      for (let k = -r; k <= r; k++) s += F[y * NW + lim(x + k, 0, NW - 1)];
      aux[y * NW + x] = s / n;
    }
    for (let y = 0; y < NH; y++) for (let x = 0; x < NW; x++) {
      let s = 0;
      for (let k = -r; k <= r; k++) s += aux[lim(y + k, 0, NH - 1) * NW + x];
      F[y * NW + x] = s / n;
    }
  }
  // Celdas con logo encima en su columna (el hueco de abajo y bajo los pies): ahí no hay fuego
  const DEBAJO = (() => {
    const D = new Float32Array(NW * NH);
    for (let x = 0; x < NW; x++) {
      let hay = 0;
      for (let y = 0; y < NH; y++) { D[y * NW + x] = hay; hay = Math.max(hay, SILUETA[y * NW + x]); }
    }
    desenfocar(D, 3);
    return D;
  })();
  // Ruido de Perlin que se repite cada 256 (se desplaza sin costuras), ecualizado a 0..255
  function textura(semilla, p0, octavas) {
    let s = semilla >>> 0;
    const rnd = () => (s = (s * 1664525 + 1013904223) >>> 0) / 4294967296;
    const N = 256, f = new Float32Array(N * N), fade = (t) => t * t * t * (t * (t * 6 - 15) + 10);
    for (let o = 0, p = p0, amp = 1; o < octavas; o++, p *= 2, amp *= .5) {
      const gx = new Float32Array(p * p), gy = new Float32Array(p * p);
      for (let i = 0; i < p * p; i++) { const a = rnd() * 2 * Math.PI; gx[i] = Math.cos(a); gy[i] = Math.sin(a); }
      const paso = N / p;
      for (let y = 0; y < N; y++) {
        const fy = y / paso, yi = Math.floor(fy), dy = fy - yi, v = fade(dy), j0 = (yi % p) * p, j1 = ((yi + 1) % p) * p;
        for (let x = 0; x < N; x++) {
          const fx = x / paso, xi = Math.floor(fx), dx = fx - xi, u = fade(dx), i0 = xi % p, i1 = (xi + 1) % p;
          const n00 = gx[j0 + i0] * dx + gy[j0 + i0] * dy, n10 = gx[j0 + i1] * (dx - 1) + gy[j0 + i1] * dy;
          const n01 = gx[j1 + i0] * dx + gy[j1 + i0] * (dy - 1), n11 = gx[j1 + i1] * (dx - 1) + gy[j1 + i1] * (dy - 1);
          const a = n00 + (n10 - n00) * u, b = n01 + (n11 - n01) * u;
          f[y * N + x] += amp * (a + (b - a) * v);
        }
      }
    }
    const orden = Array.from({ length: N * N }, (_, i) => i).sort((a, b) => f[a] - f[b]);
    const t = new Uint8Array(N * N);
    orden.forEach((i, r) => { t[i] = (r * 256 / (N * N)) | 0; });
    return t;
  }
  const RUIDO = textura(7, 4, 5), GIRO = textura(19, 2, 3);
  // Envolvente (0..255): la silueta arrastrada hacia arriba y hacia el centro (en cono),
  // suavizada y subida `sube` celdas; dentro del logo y debajo de él casi nada (o nada); se
  // apaga en los bordes de la capa
  function envolvente(largo, cono, radio, dentro, sube = 0) {
    const d = Math.exp(-NZ.c / largo), S = new Float32Array(NW * NH), xc = NW / 2;
    const en = (y, x) => (x >= 0 && x < NW ? S[y * NW + x] : 0);
    for (let y = NH - 1; y >= 0; y--) for (let x = 0; x < NW; x++) {
      let previo = 0;
      if (y < NH - 1) {
        const xs = xc + (x - xc) * (1 + cono), x0 = Math.floor(xs), f = xs - x0;
        previo = en(y + 1, x0) * (1 - f) + en(y + 1, x0 + 1) * f;
      }
      S[y * NW + x] = Math.max(SILUETA[y * NW + x], previo * d);
    }
    desenfocar(S, radio); desenfocar(S, radio);
    if (sube) S.copyWithin(0, sube * NW).fill(0, (NH - sube) * NW);
    for (let i = 0; i < NW * NH; i++) {
      const y = Math.floor(i / NW);
      S[i] *= (1 - dentro * Math.max(SILUETA[i], DEBAJO[i])) * lim(y / 30) * lim((NH - 1 - y) / 20);
    }
    return Uint8Array.from(S, (v) => Math.round(lim(v) * 255));
  }
  // la nube nunca encima del logo ni a los lados de los pies: empieza donde acaban las llamas
  const CERCA = envolvente(38, .004, 2, .85), NUBE = envolvente(230, .006, 6, 1, 14);
  // Paletas de 256 colores (se suman: el negro no cuenta)
  function rampa(paradas) {
    const t = new Uint8Array(256 * 3);
    for (let v = 0; v < 256; v++) {
      let k = 0;
      while (k < paradas.length - 2 && v > paradas[k + 1][0]) k++;
      const [v0, ...c0] = paradas[k], [v1, ...c1] = paradas[k + 1], f = lim((v - v0) / (v1 - v0));
      for (let j = 0; j < 3; j++) t[v * 3 + j] = Math.round(c0[j] + (c1[j] - c0[j]) * f);
    }
    return t;
  }
  const FUEGO = rampa([[0, 0, 0, 0], [40, 18, 0, 44], [95, 64, 10, 150], [145, 116, 48, 230], [190, 155, 92, 255], [228, 204, 166, 255], [255, 244, 236, 255]]);
  const HUMO_N = rampa([[0, 0, 0, 0], [90, 24, 4, 56], [170, 54, 16, 118], [255, 88, 36, 176]]);
  const capaN = nuevo(NW, NH), cgN = capaN.getContext('2d'), imgN = cgN.createImageData(NW, NH), pxN = imgN.data;
  for (let i = 3; i < pxN.length; i += 4) pxN[i] = 255;
  const idx = (fila, col) => ((fila & 255) << 8) | (col & 255);

  // Se enciende poco a poco: primero prende junto al borde (donde la envolvente es alta) y el
  // frente baja con ruido hasta dejarla entera; la nube sale cuando ya hay llamas
  function pintarNublado(e) {
    const enc = suave(st.encendido), encN = suave((st.encendido - .35) / .65);
    if (e.fx <= .003 || enc <= .003) return;
    const t = st.t, fx = e.fx, umbralC = (1 - enc) * 320, umbralN = (1 - encN) * 320;
    const sube = (t * 200) | 0, lento = (t * 46) | 0, g1 = (t * 16) | 0, g2 = (t * 11) | 0;
    for (let y = 0; y < NH; y++) for (let x = 0; x < NW; x++) {
      // dos giros encadenados: la nube se enrosca
      const w1 = GIRO[idx(y + g1, x)] - 128, w2 = GIRO[idx(x + 97 + g2, y + (w1 >> 3))] - 128;
      const xs = lim(x + (w1 >> 3), 0, NW - 1), ys = lim(y + (w2 >> 4), 0, NH - 1);
      const n2 = RUIDO[idx(y * 2 + sube, x * 4)], ec = CERCA[y * NW + x], en = NUBE[ys * NW + xs];
      const kc = (ec - umbralC + (n2 - 128) * .5) / 50, kn = (en - umbralN) / 80;
      const cerca = kc > 0 ? ec * fx * Math.min(1, kc) : 0, nube = kn > 0 ? en * fx * Math.min(1, kn) : 0;
      const humo = nube > 3 ? nube * RUIDO[idx(ys + lento, xs * 2)] / 255 * 1.9 - 30 : 0;
      const fuego = cerca > 3 ? cerca * 1.5 - (255 - n2) * .95 : 0;
      const a = lim(fuego | 0, 0, 255) * 3, b = lim(humo | 0, 0, 255) * 3, o = (y * NW + x) * 4;
      pxN[o] = FUEGO[a] + HUMO_N[b]; pxN[o + 1] = FUEGO[a + 1] + HUMO_N[b + 1]; pxN[o + 2] = FUEGO[a + 2] + HUMO_N[b + 2];
    }
    cgN.putImageData(imgN, 0, 0);
    ctx.globalCompositeOperation = 'lighter'; ctx.globalAlpha = 1; ctx.imageSmoothingEnabled = true;
    ctx.drawImage(capaN, e.x + NZ.x * e.esc, e.y + NZ.y * e.esc, NZ.w * e.esc, NZ.h * e.esc);
  }

  // ================================================================ estado
  const chispas = [], llamas = [], ondas = [], subiendo = [];
  // Humo que deriva: cada bocanada da vueltas lentas alrededor de un punto del borde
  const DERIVA = [[170, 150, 'rojo'], [1750, 170, 'morado'], [110, 900, 'morado'], [1810, 930, 'rojo'], [960, 1130, 'rojo'],
    [960, -70, 'morado'], [560, 1060, 'morado'], [1380, 30, 'rojo']].map(([x, y, col], i) => ({
    x, y, col, spr: i % 3, ax: azar(140, 300), ay: azar(60, 170), fx: azar(.012, .028), fy: azar(.01, .024),
    fase: azar(0, 6.28), giro: azar(-.035, .035), g0: azar(0, 6.28), esc: azar(2.3, 3.2), alfa: azar(.45, .7),
  }));
  const st = {
    t: 0, reloj: 0, elegido: 'windows', anim: {}, peso: {}, auraX: 0,
    cuenta: 10, cuentaActiva: true, cuentaAlfa: 1,
    menu: { abierto: false, m: 0, i: 0 }, arranque: null, aviso: null,
    acum: { arch: 0, kali: 0 }, ondaArch: 0, encendido: 0,
    glitch: { hasta: 0, proximo: 1.5, refresco: 0, activo: false, cortes: [], fx: 8, aura: 1 },
  };

  function reiniciar() {
    Object.assign(st, { reloj: 0, elegido: 'windows', cuenta: 10, cuentaActiva: true, cuentaAlfa: 1, arranque: null, aviso: null, encendido: 0 });
    Object.assign(st.menu, { abierto: false, m: 0, i: 0 });
    for (const id of IDS) st.anim[id] = st.peso[id] = id === 'windows' ? 1 : 0;
    st.auraX = posX('windows');
    chispas.length = 0; llamas.length = 0; ondas.length = 0;
  }

  // ================================================================ partículas
  function chispa(x, y, pal, o) {
    chispas.push({ x, y, vx: o.vx ?? 0, vy: o.vy, fase: azar(0, 6.28), amp: o.amp ?? azar(3, 10), frec: azar(1, 2.5), t: 0, vida: o.vida, tam: o.tam, parpadeo: azar(0, 6.28), pal });
  }
  function moverBrasas(lista, dt) {
    const freno = Math.pow(.3, dt);
    for (let i = lista.length - 1; i >= 0; i--) {
      const p = lista[i];
      p.t += dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vx *= freno;
      if (p.t >= p.vida || p.y < -30) lista.splice(i, 1);
    }
  }
  function humoNuevo(t0) {
    return {
      x: azar(80, W - 80), y: H + 230, vy: -azar(22, 42), t: t0, vida: azar(16, 26), spr: (Math.random() * 3) | 0,
      col: Math.random() < .5 ? 'rojo' : 'morado', g0: azar(0, 6.28), giro: azar(-.05, .05),
      esc0: azar(1.5, 2.1), esc1: azar(2.9, 3.9), alfa: azar(.6, .9), fase: azar(0, 6.28),
    };
  }
  function moverFondo(dt) {
    if (cfg.fondo === 'sube' && subiendo.length < 10 && Math.random() < dt * .7) subiendo.push(humoNuevo(0));
    for (let i = subiendo.length - 1; i >= 0; i--) {
      const p = subiendo[i];
      p.t += dt; p.y += p.vy * dt;
      if (p.t > p.vida) subiendo.splice(i, 1);
    }
  }
  // Cuántas partículas salen por segundo del borde de un logo
  function emitir(id, porSegundo, dt, crear) {
    st.acum[id] += porSegundo * dt;
    const e = estadoDe(id), pts = BORDES[id];
    while (st.acum[id] >= 1) {
      st.acum[id] -= 1;
      const p = pts[(Math.random() * pts.length) | 0];
      crear(e.x + (p[0] + azar(-2, 2)) * e.esc, e.y + p[1] * e.esc, e.esc);
    }
  }
  const latido = (t) => { const f = t % 1.5, g = (c) => Math.exp(-Math.pow((f - c) / .075, 2)); return g(.1) + .65 * g(.36); };

  // ================================================================ lógica
  function arrancar(id) {
    st.arranque = { id, t: 0 }; st.menu.abierto = false; st.cuentaActiva = false;
    if (cfg.lanzar === 'propio') return;
    const e = estadoDe(id);
    for (let i = 0; i < 70; i++) chispa(e.x + azar(-70, 70), e.y + azar(-70, 70), id, { vx: azar(-240, 240), vy: -azar(40, 320), vida: azar(.8, 1.7), tam: azar(4, 12) });
  }
  function avisar(texto) { st.aviso = { texto, t: 0 }; }

  function actualizar(dt) {
    st.t += dt; st.reloj += dt;
    for (const id of IDS) {
      const obj = id === st.elegido ? 1 : 0;
      st.anim[id] = aprox(st.anim[id], obj, 11, dt);
      st.peso[id] = aprox(st.peso[id], obj, 8, dt);
    }
    st.auraX = aprox(st.auraX, posX(st.elegido), 10, dt);
    // el fuego de Arch tarda 1,6 s en prender del todo y medio en apagarse
    st.encendido = lim(st.encendido + (st.elegido === 'arch' ? dt / 1.6 : -dt / .5));
    st.menu.m = aprox(st.menu.m, st.menu.abierto ? 1 : 0, 14, dt);
    st.cuentaAlfa = aprox(st.cuentaAlfa, st.cuentaActiva ? 1 : 0, 10, dt);
    if (st.cuentaActiva && st.reloj > .8 && !st.arranque) {
      st.cuenta -= dt;
      if (st.cuenta <= 0) { st.cuenta = 0; arrancar(st.elegido); }
    }
    if (st.aviso && (st.aviso.t += dt) > 2.4) st.aviso = null;
    if (st.arranque && (st.arranque.t += dt) > duracion(st.arranque.id) + 2.7) reiniciar();

    moverFondo(dt);
    if (cfg.arch === 'llama') emitir('arch', 300 * st.anim.arch, dt, (x, y, esc) => llamas.push({
      x, y, vx: azar(-10, 10), vy: -azar(60, 130), t: 0, vida: azar(.32, .7), tam: azar(7, 13) * esc,
      fase: azar(0, 6.28), frec: azar(3, 6), amp: azar(1.5, 4.5),
    }));
    for (let i = llamas.length - 1; i >= 0; i--) {
      const p = llamas[i];
      p.t += dt; p.vy -= 160 * dt; p.x += p.vx * dt; p.y += p.vy * dt; // el calor la empuja hacia arriba
      if (p.t >= p.vida) llamas.splice(i, 1);
    }
    if (cfg.kali === 'brasas') emitir('kali', 32 * st.anim.kali, dt, (x, y) => chispa(x, y, 'kali', { vy: -azar(30, 85), vx: azar(-10, 10), vida: azar(1.2, 2.6), tam: azar(3, 9) }));
    moverBrasas(chispas, dt);

    if (cfg.arch === 'ondas' && st.elegido === 'arch') {
      if ((st.ondaArch -= dt) <= 0) { st.ondaArch = 1.35; ondas.push({ id: 'arch', t: 0, dur: 2.3, r0: 110, r1: 360, alfa: .55 }); }
    } else st.ondaArch = 0;
    if (cfg.kali === 'latido' && st.elegido === 'kali') {
      const cruza = (c) => Math.floor((st.t - c) / 1.5) !== Math.floor((st.t - dt - c) / 1.5);
      if (cruza(.1)) ondas.push({ id: 'kali', t: 0, dur: 1.1, r0: 100, r1: 300, alfa: .6 });
      if (cruza(.36)) ondas.push({ id: 'kali', t: 0, dur: 1.1, r0: 100, r1: 260, alfa: .35 });
    }
    for (let i = ondas.length - 1; i >= 0; i--) if ((ondas[i].t += dt) > ondas[i].dur) ondas.splice(i, 1);

    const g = st.glitch;
    g.activo = false;
    if (cfg.kali === 'glitch') {
      if (st.t >= g.proximo) { g.hasta = st.t + azar(.12, .3); g.proximo = st.t + azar(1.3, 3.4); }
      g.activo = st.t < g.hasta && st.anim.kali > .3;
      // al lanzar Kali, el logo se glitchea sin parar
      if (cfg.lanzar === 'propio' && st.arranque?.id === 'kali' && st.arranque.t < 1) g.activo = true;
      if (g.activo && st.t >= g.refresco) {
        g.refresco = st.t + .045; g.fx = azar(5, 13); g.aura = azar(.35, 1.2); g.cortes = [];
        for (let y = 0; y < L;) { const h = azar(6, 46); g.cortes.push({ y, h, dx: Math.random() < .45 ? azar(-30, 30) : 0 }); y += h; }
      }
    }
  }

  // Posición, escala y alfa de un sistema en este fotograma (entrada, elección y arranque)
  function estadoDe(id) {
    const i = ORDENES[cfg.orden].indexOf(id), a = st.anim[id], arr = st.arranque;
    const vis = salida((st.reloj - .1 - i * .09) / .6);
    let esc = .74 + .26 * a, alfa = (.42 + .58 * a) * vis, fx = a * vis;
    if (arr) {
      const propio = cfg.lanzar === 'propio';
      if (id !== arr.id) { const k = 1 - lim(arr.t / .35); alfa *= k; fx *= k; }
      else if (propio && id === 'windows') { alfa = 0; fx = 0; } // lo pintan sus cristales al abrirse
      else if (!propio || id === 'arch') esc *= 1 + .2 * suave(arr.t / .9);
    }
    if (id === 'kali' && cfg.kali === 'latido') esc *= 1 + .05 * latido(st.t) * a;
    return { x: posX(id), y: Y + (1 - vis) * 26, esc, alfa, a, vis, fx };
  }

  function intensidadAura(id) {
    if (id === 'kali' && cfg.kali === 'latido') return .5 + .6 * latido(st.t);
    if (id === 'kali' && st.glitch.activo) return st.glitch.aura;
    return .8 + .2 * Math.sin(st.t * 2 * Math.PI / 3.4);
  }

  // ================================================================ dibujo
  function centrado(spr, x, y, esc = 1, alfa = 1) {
    if (alfa <= .003) return;
    ctx.globalAlpha = Math.min(1, alfa);
    const w = spr.width * esc, h = spr.height * esc;
    ctx.drawImage(spr, x - w / 2, y - h / 2, w, h);
  }
  function girado(spr, x, y, esc, giro, alfa) {
    if (alfa <= .003) return;
    ctx.save(); ctx.translate(x, y); ctx.rotate(giro);
    centrado(spr, 0, 0, esc, alfa);
    ctx.restore();
  }

  function pintarFondo() {
    ctx.globalCompositeOperation = 'lighter';
    const t = st.t;
    if (cfg.fondo === 'bordes') {
      // dos sombras que respiran a destiempo y se mecen un poco
      ctx.imageSmoothingQuality = 'high';
      for (const [col, d] of [['rojo', 1], ['morado', -1]]) {
        ctx.globalAlpha = .45 + .2 * Math.sin(t * .7 * d);
        ctx.drawImage(SPR.borde[col], Math.sin(t * .23) * 26 * d - 30, Math.cos(t * .19) * 16 - 20, W + 60, H + 40);
      }
    }
    if (cfg.fondo === 'humo' || cfg.fondo === 'bordes') {
      const k = cfg.fondo === 'bordes' ? .5 : 1;
      for (const h of DERIVA) {
        const x = h.x + Math.sin(t * 6.28 * h.fx + h.fase) * h.ax, y = h.y + Math.cos(t * 6.28 * h.fy + h.fase * 1.3) * h.ay;
        girado(SPR.humo[h.spr][h.col], x, y, h.esc, h.g0 + t * h.giro, k * h.alfa * (.75 + .25 * Math.sin(t * .4 + h.fase)));
      }
    }
    if (cfg.fondo === 'sube') {
      for (const p of subiendo) {
        const f = p.t / p.vida;
        girado(SPR.humo[p.spr][p.col], p.x + Math.sin(p.t * .3 + p.fase) * 60, p.y, p.esc0 + (p.esc1 - p.esc0) * f, p.g0 + p.t * p.giro, p.alfa * Math.sin(f * Math.PI));
      }
    }
  }

  function pintarBrasas(lista) {
    ctx.globalCompositeOperation = 'lighter';
    for (const p of lista) {
      const vida = p.t / p.vida;
      const x = p.x + Math.sin(p.fase + p.t * p.frec * 2.4) * p.amp * Math.min(1, p.t);
      const alfa = Math.min(1, p.t * 3) * (1 - vida) * (.7 + .3 * Math.sin(p.parpadeo + p.t * 9));
      if (alfa <= 0) continue;
      const s = p.tam * (1 - vida * .5);
      ctx.globalAlpha = alfa;
      ctx.drawImage(SPR.brasa[p.pal][vida < .3 ? 0 : vida < .65 ? 1 : 2], x - s, p.y - s, s * 2, s * 2);
    }
  }

  // Cada lengua nace ancha y caliente en el borde, se estira al subir, se afina y se enfría
  function pintarLlamas(fx) {
    if (fx <= .01 && !llamas.length) return;
    ctx.globalCompositeOperation = 'lighter';
    for (const p of llamas) {
      const v = p.t / p.vida;
      const w = p.tam * (1 - .55 * v), h = w * (2.1 + 1.8 * v);
      const x = p.x + Math.sin(p.fase + p.t * p.frec * 6.28) * p.amp;
      ctx.globalAlpha = lim(Math.min(1, p.t * 14) * Math.pow(1 - v, 1.2) * (.75 + .25 * Math.sin(p.fase * 3 + p.t * 31)) * Math.max(fx, .25));
      ctx.drawImage(SPR.lengua[v < .22 ? 0 : v < .55 ? 1 : 2], x - w / 2, p.y - h * .86, w, h);
    }
  }

  // Dibuja algo recortado a la silueta de un logo (destello de Windows, barrido de Kali)
  function enSilueta(logo, e, alfa, dibujar) {
    if (alfa <= .003) return;
    tg.globalCompositeOperation = 'source-over'; tg.globalAlpha = 1; tg.clearRect(0, 0, L, L);
    dibujar(tg);
    tg.globalCompositeOperation = 'destination-in'; tg.drawImage(logo, 0, 0);
    ctx.globalCompositeOperation = 'lighter';
    centrado(tmp, e.x, e.y, e.esc, alfa);
  }

  function pintarOrbita(e, delante) {
    if (e.fx <= .01) return;
    ctx.globalCompositeOperation = 'lighter';
    const inc = -.22, ci = Math.cos(inc), si = Math.sin(inc);
    for (let s = 0; s < 3; s++) {
      const base = st.t * 2 * Math.PI / 3.4 + s * 2 * Math.PI / 3;
      for (let k = 15; k >= 0; k--) {
        const ang = base - k * .05;
        if ((Math.sin(ang) > 0) !== delante) continue;
        const px = Math.cos(ang) * 178 * e.esc, py = Math.sin(ang) * 50 * e.esc;
        const f = 1 - k / 16, tam = (k === 0 ? 15 : 3 + 9 * f) * e.esc;
        ctx.globalAlpha = Math.pow(f, 1.6) * e.fx * (delante ? 1 : .5);
        ctx.drawImage(SPR.brasa.arch[k < 3 ? 0 : 1], e.x + px * ci - py * si - tam, e.y + px * si + py * ci - tam, tam * 2, tam * 2);
      }
    }
  }

  function pintarGlitch(e) {
    const s = e.esc, x0 = e.x - L / 2 * s, y0 = e.y - L / 2 * s, g = st.glitch;
    ctx.globalCompositeOperation = 'lighter';
    ctx.globalAlpha = .55 * e.alfa; ctx.drawImage(SPR.fantasma[0], x0 + g.fx * s, y0, L * s, L * s);
    ctx.globalAlpha = .5 * e.alfa; ctx.drawImage(SPR.fantasma[1], x0 - g.fx * .8 * s, y0 + 2, L * s, L * s);
    ctx.globalCompositeOperation = 'source-over'; ctx.globalAlpha = e.alfa;
    for (const c of g.cortes) ctx.drawImage(SPR.logo.kali, 0, c.y, L, c.h, x0 + c.dx * s, y0 + c.y * s, L * s, c.h * s);
  }

  function pintarSistema(id) {
    const e = estadoDe(id), t = st.t;
    // detrás del logo
    ctx.globalCompositeOperation = 'lighter';
    if (id === 'windows' && cfg.windows === 'rayos' && e.fx > .01) girado(SPR.rayos, e.x, e.y, e.esc, t * .12, .4 * e.fx * (.85 + .15 * Math.sin(t * 1.8)));
    for (const o of ondas) if (o.id === id) {
      const p = o.t / o.dur, r = o.r0 + (o.r1 - o.r0) * salida(p);
      centrado(SPR.anillo[id], e.x, e.y, r / 300 * e.esc, o.alfa * (1 - p) * e.vis);
    }
    if (id === 'arch' && cfg.arch === 'orbita') pintarOrbita(e, false);

    // logo
    ctx.globalCompositeOperation = 'source-over';
    if (id === 'kali' && st.glitch.activo) pintarGlitch(e);
    else centrado(SPR.logo[id], e.x, e.y, e.esc, e.alfa);

    // encima del logo
    if (id === 'windows' && cfg.windows === 'respira') {
      const f = ((t + 2) % 5.5) / .95;
      if (f < 1) enSilueta(SPR.logo.windows, e, .75 * e.fx, (g) => g.drawImage(SPR.brillo, L / 2 - 300 - 330 + 660 * suave(f), L / 2 - 300));
    }
    if (id === 'windows' && cfg.windows === 'cuadros') {
      ctx.globalCompositeOperation = 'lighter';
      SPR.cuadros.forEach((spr, i) => {
        const f = (((t - i * .3) % 2.4) + 2.4) % 2.4;
        if (f < .6) centrado(spr, e.x, e.y, e.esc, .5 * Math.sin(f / .6 * Math.PI) * e.fx);
      });
    }
    if (id === 'kali' && cfg.kali === 'glitch') {
      ctx.globalCompositeOperation = 'source-over';
      centrado(SPR.lineas, e.x, e.y, e.esc, e.fx);
      const f = (t % 2.8) / 1.1;
      if (f < 1) enSilueta(SPR.logo.kali, e, .8 * e.fx, (g) => g.drawImage(SPR.barrido, 0, -36 + (L + 36) * f));
    }
    if (id === 'arch' && cfg.arch === 'orbita') pintarOrbita(e, true);
    if (id === 'arch') pintarLlamas(e.fx);
    if (id === 'arch' && cfg.arch === 'nublado') pintarNublado(e);
  }

  function pintarNombre(id) {
    const e = estadoDe(id), arr = st.arranque;
    const k = e.vis * (arr ? 1 - lim(arr.t / .3) : 1);
    if (k <= .003) return;
    ctx.globalCompositeOperation = 'source-over';
    ctx.font = `25px Nombre-${id}`; ctx.letterSpacing = '8px'; ctx.textAlign = 'center'; ctx.textBaseline = 'alphabetic';
    ctx.fillStyle = '#ece8f2'; ctx.globalAlpha = (.3 + .7 * e.a) * k;
    ctx.fillText(SISTEMAS[id].nombre, e.x + 4, e.y + 168);
    // raya de su color bajo el elegido; en el de por defecto hace de cuenta atrás
    const cuenta = id === 'windows' ? st.cuentaAlfa : 0;
    const ancho = 44 + (220 * st.cuenta / 10 - 44) * cuenta;
    ctx.globalAlpha = Math.max(e.a, cuenta) * k; ctx.fillStyle = rgba(SISTEMAS[id].logo, 1);
    ctx.fillRect(e.x - ancho / 2, e.y + 188, ancho, 2);
    if (cuenta > .01) {
      ctx.font = '500 20px Texto'; ctx.letterSpacing = '2px'; ctx.fillStyle = '#ece8f2';
      ctx.globalAlpha = .5 * cuenta * k;
      ctx.fillText(`Se inicia en ${Math.max(1, Math.ceil(st.cuenta))} s`, e.x + 1, e.y + 224);
    }
    ctx.letterSpacing = '0px';
  }

  function pintarMenu() {
    const m = st.menu.m;
    if (m < .01) return;
    ctx.globalCompositeOperation = 'source-over';
    ctx.globalAlpha = .74 * m; ctx.fillStyle = '#000'; ctx.fillRect(0, 0, W, H);
    const col = SISTEMAS[st.elegido].logo, n = OPCIONES.length, alto = 78, ancho = 640;
    const x0 = W / 2 - ancho / 2, y0 = H / 2 - n * alto / 2 + 30 + (1 - m) * 18;
    ctx.globalAlpha = m;
    ctx.fillStyle = 'rgba(10,8,14,.9)'; ctx.strokeStyle = 'rgba(255,255,255,.08)'; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.roundRect(x0 - 26, y0 - 96, ancho + 52, n * alto + 172, 18); ctx.fill(); ctx.stroke();
    ctx.textBaseline = 'alphabetic';
    ctx.font = '500 19px Texto'; ctx.letterSpacing = '8px'; ctx.textAlign = 'center'; ctx.fillStyle = 'rgba(236,232,242,.5)';
    ctx.fillText('MÁS OPCIONES', W / 2 + 4, y0 - 44);
    ctx.letterSpacing = '0px';
    OPCIONES.forEach((o, i) => {
      const y = y0 + i * alto, sel = i === st.menu.i;
      if (sel) {
        ctx.fillStyle = rgba(col, .16); ctx.beginPath(); ctx.roundRect(x0, y + 4, ancho, alto - 8, 12); ctx.fill();
        ctx.fillStyle = rgba(col, 1); ctx.fillRect(x0, y + 22, 4, alto - 44);
      }
      ctx.textAlign = 'center'; ctx.textBaseline = 'middle'; ctx.font = '32px Simbolos';
      ctx.fillStyle = sel ? rgba(aclarar(col, .2), 1) : 'rgba(255,255,255,.42)';
      ctx.fillText(String.fromCodePoint(o.icono), x0 + 46, y + alto / 2);
      ctx.textAlign = 'left'; ctx.textBaseline = 'alphabetic'; ctx.font = '500 26px Texto';
      ctx.fillStyle = sel ? '#f3eff8' : 'rgba(243,239,248,.62)';
      ctx.fillText(o.nombre, x0 + 92, y + (o.detalle ? 36 : 48));
      if (o.detalle) {
        ctx.font = '500 18px Texto'; ctx.fillStyle = sel ? 'rgba(243,239,248,.55)' : 'rgba(243,239,248,.3)';
        ctx.fillText(o.detalle, x0 + 92, y + 61);
      }
    });
    ctx.textAlign = 'center'; ctx.font = '500 18px Texto'; ctx.letterSpacing = '2px'; ctx.fillStyle = 'rgba(236,232,242,.38)';
    ctx.fillText('Esc  volver     ·     Intro  abrir', W / 2, y0 + n * alto + 50);
    ctx.letterSpacing = '0px';
  }

  function pintarTexto(texto, y, alfa, tam = 24) {
    if (alfa <= .003) return;
    ctx.globalCompositeOperation = 'source-over'; ctx.globalAlpha = alfa;
    ctx.font = `500 ${tam}px Texto`; ctx.letterSpacing = '1px'; ctx.textAlign = 'center'; ctx.textBaseline = 'alphabetic';
    ctx.fillStyle = '#ece8f2'; ctx.fillText(texto, W / 2, y);
    ctx.letterSpacing = '0px';
  }

  // ================================================================ lanzar cada sistema (cfg.lanzar === 'propio')
  // Kali: toda la pantalla se glitchea cada vez más y se apaga como una tele vieja.
  // Windows: los cuatro cristales se abren hacia fuera mientras te acercas y entras en la luz.
  // Arch: la pantalla arde como un papel desde la A.
  // En la app: franjas de filas desplazadas, canales corridos, rectángulos y la capa de ¼.
  const DURACION = { kali: 1.35, windows: 1.45, arch: 1.8 };
  const duracion = (id) => (cfg.lanzar === 'propio' ? DURACION[id] : 1.12);
  const copia = nuevo(W, H), cc = copia.getContext('2d'), tinte = nuevo(W, H), ct = tinte.getContext('2d');
  const RAYAS = (() => {
    const c = nuevo(W, 4), g = c.getContext('2d');
    g.fillStyle = '#000'; g.fillRect(0, 0, W, 2);
    return ctx.createPattern(c, 'repeat');
  })();

  function lanzarKali(t) {
    cc.globalCompositeOperation = 'copy'; cc.drawImage(lienzo, 0, 0); // el fotograma tal cual
    ctx.globalAlpha = 1;
    if (t < 1) {
      const g = suave(t / .7);
      // franjas de la pantalla corridas de lado
      ctx.globalCompositeOperation = 'source-over';
      for (let i = 0, n = 4 + ((g * 26) | 0); i < n; i++) {
        const y = Math.random() * H, h = azar(3, 18 + 70 * g), dx = azar(-1, 1) * (12 + 160 * g);
        ctx.drawImage(copia, 0, y, W, h, dx, y, W, h);
      }
      // fantasmas rojo y morado de toda la pantalla
      for (const [color, dx] of [['rgb(255,40,70)', 6 + 26 * g], ['rgb(110,20,255)', -(5 + 22 * g)]]) {
        ct.globalCompositeOperation = 'copy'; ct.drawImage(copia, 0, 0);
        ct.globalCompositeOperation = 'multiply'; ct.fillStyle = color; ct.fillRect(0, 0, W, H);
        ctx.globalCompositeOperation = 'lighter'; ctx.globalAlpha = .55 * g;
        ctx.drawImage(tinte, dx, azar(-2, 2) * g);
      }
      // rayas de tele y algún destello
      ctx.globalCompositeOperation = 'source-over'; ctx.globalAlpha = .35 * g;
      ctx.fillStyle = RAYAS; ctx.fillRect(0, 0, W, H);
      if (Math.random() < .2 * g) {
        ctx.globalAlpha = azar(.08, .22); ctx.fillStyle = Math.random() < .5 ? '#c0182f' : '#e8e0ff';
        ctx.fillRect(0, azar(0, H), W, azar(4, 90));
      }
      return;
    }
    // se apaga como una tele vieja: la imagen se aplasta en una raya y la raya en un punto
    const k = (t - 1) / .3;
    ctx.globalCompositeOperation = 'source-over'; ctx.fillStyle = '#000'; ctx.fillRect(0, 0, W, H);
    if (k < .55) {
      const h = Math.max(2, H * (1 - suave(k / .55)));
      ctx.drawImage(copia, 0, (H - h) / 2, W, h);
      ctx.globalCompositeOperation = 'lighter'; ctx.globalAlpha = .3 * k / .55;
      ctx.fillStyle = '#ffd6dc'; ctx.fillRect(0, (H - h) / 2, W, h);
    } else if (k < 1) {
      const w = Math.max(2, W * (1 - suave((k - .55) / .4)));
      ctx.fillStyle = '#ffe8ec'; ctx.fillRect((W - w) / 2, H / 2 - 1, w, 2);
    }
  }

  function lanzarWindows(t) {
    const e = estadoDe('windows'), p = lim(t / 1.25);
    const zoom = 1 + 11 * Math.pow(p, 2.6), abre = 520 * Math.pow(p, 2.2);
    // la luz de detrás de la ventana crece desde el centro
    const luz = suave(p / .8) * (1 - lim((t - 1.2) / .25)), r = 60 + 1500 * p * p;
    const gr = ctx.createRadialGradient(e.x, e.y, 0, e.x, e.y, r);
    gr.addColorStop(0, rgba([235, 245, 255], .95 * luz)); gr.addColorStop(.35, rgba([90, 170, 255], .6 * luz)); gr.addColorStop(1, 'rgba(20,80,230,0)');
    ctx.globalCompositeOperation = 'lighter'; ctx.globalAlpha = 1; ctx.fillStyle = gr; ctx.fillRect(0, 0, W, H);
    // los cuatro cristales se separan hacia sus esquinas y se agrandan
    ctx.globalCompositeOperation = 'source-over'; ctx.globalAlpha = 1 - lim((p - .75) / .25);
    ctx.fillStyle = rgba(aclarar(SISTEMAS.windows.logo, .25 * p), 1);
    for (const [x, y, w, h] of SISTEMAS.windows.cuadros) {
      const cx = x + w / 2 - L / 2, cy = y + h / 2 - L / 2;
      const px = e.x + cx * zoom + Math.sign(cx) * abre, py = e.y + cy * zoom + Math.sign(cy) * abre;
      ctx.fillRect(px - w * zoom / 2, py - h * zoom / 2, w * zoom, h * zoom);
    }
  }

  // Arch: la pantalla arde como un papel desde la A. Un agujero negro de borde irregular crece
  // desde el logo; el borde está encendido, de él suben llamas (también por encima de lo ya
  // quemado) y detrás quedan brasas moradas que se apagan. Capa de 480×270 (la de ¼ en la app).
  const FW = 480, FH = 270, BORDE = 7;
  const capaF = nuevo(FW, FH), cgF = capaF.getContext('2d'), imgF = cgF.createImageData(FW, FH), pxF = imgF.data;
  const capaQ = nuevo(FW, FH), cgQ = capaQ.getContext('2d'), imgQ = cgQ.createImageData(FW, FH), pxQ = imgQ.data;
  for (let i = 3; i < pxF.length; i += 4) pxF[i] = 255;
  const FUENTE = new Float32Array(FW * FH);
  // lo irregular del borde: fijo (no se mueve), como las fibras de un papel
  const IRREGULAR = Float32Array.from({ length: FW * FH }, (_, i) => {
    const x = i % FW, y = (i / FW) | 0;
    return (GIRO[idx(y * 2, x * 2)] - 128) * .22 + (RUIDO[idx(y * 5 + 31, x * 5)] - 128) * .07;
  });
  function lanzarArch(t) {
    const e = estadoDe('arch'), cx = e.x / 4, cy = e.y / 4;
    const radio = 470 * Math.pow(lim((t - .1) / 1.45), 1.4);
    const sube = (t * 300) | 0, sube2 = (t * 460) | 0, gira = (t * 40) | 0;
    // dónde está el borde (q de 0 a 1), lo quemado (q > 1) y lo que aún no (q < 0)
    for (let y = 0; y < FH; y++) for (let x = 0; x < FW; x++) {
      const i = y * FW + x, dx = x - cx, dy = y - cy;
      const q = (radio - Math.sqrt(dx * dx + dy * dy) - IRREGULAR[i]) / BORDE;
      FUENTE[i] = radio > 0 ? lim(1 - Math.abs(q - .4) / .8) : 0;
      pxQ[i * 4 + 3] = lim((q - .5) / .6) * 255; // negro de lo quemado
      // brasas justo detrás del borde
      const b = q > .8 ? lim(1 - (q - .8) / 2.5) * 120 : 0;
      const a = (b | 0) * 3;
      pxF[i * 4] = HUMO_N[a]; pxF[i * 4 + 1] = HUMO_N[a + 1]; pxF[i * 4 + 2] = HUMO_N[a + 2];
    }
    // las llamas: el calor del borde sube por cada columna y se va enfriando; el ruido lo
    // rompe en lenguas altas que suben deprisa
    for (let x = 0; x < FW; x++) {
      let calor = 0;
      for (let y = FH - 1; y >= 0; y--) {
        const i = y * FW + x;
        calor = Math.max(FUENTE[i] * 255, calor * .93);
        if (calor < 4) continue;
        const w = (GIRO[idx(y * 2 + gira, x * 2)] - 128) >> 4;
        const n1 = RUIDO[idx(((y * 3) >> 2) + sube, (x + w) * 2)], n2 = RUIDO[idx(y + sube2 + 128, (x + w) * 4 + 64)];
        const v = calor * 1.55 - (255 - (n1 * 2 + n2) / 3) * 1.3;
        if (v <= 0) continue;
        const a = Math.min(255, v | 0) * 3;
        pxF[i * 4] += FUEGO[a]; pxF[i * 4 + 1] += FUEGO[a + 1]; pxF[i * 4 + 2] += FUEGO[a + 2];
      }
    }
    cgQ.putImageData(imgQ, 0, 0); cgF.putImageData(imgF, 0, 0);
    ctx.imageSmoothingEnabled = true; ctx.globalAlpha = 1;
    ctx.globalCompositeOperation = 'source-over'; ctx.drawImage(capaQ, 0, 0, W, H);
    ctx.globalCompositeOperation = 'lighter'; ctx.drawImage(capaF, 0, 0, W, H);
  }

  function pintar() {
    const arr = st.arranque;
    ctx.globalCompositeOperation = 'source-over'; ctx.globalAlpha = 1;
    ctx.fillStyle = '#000'; ctx.fillRect(0, 0, W, H);
    pintarFondo();

    // aura: se desliza hasta el elegido y cambia de color por el camino
    ctx.globalCompositeOperation = 'lighter';
    const elegido = estadoDe(st.elegido);
    for (const id of IDS) {
      let k = st.peso[id] * intensidadAura(id) * elegido.vis;
      if (arr) k *= arr.id === id ? 1 + .9 * lim(arr.t / .5) : 1 - lim(arr.t / .35);
      centrado(SPR.aura[id], st.auraX, elegido.y, elegido.esc, k);
    }
    for (const id of IDS) pintarSistema(id);
    pintarBrasas(chispas);
    for (const id of IDS) pintarNombre(id);
    if (cfg.pista === 'si') {
      ctx.globalCompositeOperation = 'source-over';
      centrado(SPR.pista, W / 2, H - 78, 1, .3 * lim((st.reloj - .6) / .6) * (1 - st.menu.m) * (arr ? 1 - lim(arr.t / .3) : 1));
    }
    pintarMenu();
    const propio = arr && cfg.lanzar === 'propio';
    if (propio && arr.id === 'windows') lanzarWindows(arr.t);
    if (propio && arr.id === 'arch') lanzarArch(arr.t);
    if (propio && arr.id === 'kali') lanzarKali(arr.t);

    // fundidos a negro: al entrar y al arrancar
    const fin = arr ? duracion(arr.id) : 0;
    const negro = Math.max(1 - lim(st.reloj / .7), arr ? (propio ? lim((arr.t - fin + (arr.id === 'kali' ? .05 : .35)) / (arr.id === 'kali' ? .05 : .35)) : lim((arr.t - .45) / .65)) : 0);
    if (negro > .003) { ctx.globalCompositeOperation = 'source-over'; ctx.globalAlpha = negro; ctx.fillStyle = '#000'; ctx.fillRect(0, 0, W, H); }
    if (arr && arr.t > fin + .03) {
      const k = lim((arr.t - fin - .03) / .3) * (1 - lim((arr.t - fin - 2.2) / .4));
      pintarTexto(`(maqueta) Aquí arrancaría ${NOMBRE[arr.id]}`, H / 2 - 6, .75 * k, 28);
      pintarTexto(SISTEMAS[arr.id].destino, H / 2 + 36, .4 * k, 20);
    }
    if (st.aviso) pintarTexto(`(maqueta) ${st.aviso.texto}`, H - 150, .8 * lim(st.aviso.t / .2) * (1 - lim((st.aviso.t - 2) / .4)));
  }

  // ================================================================ teclado y panel
  addEventListener('keydown', (ev) => {
    const k = ev.key;
    if (k === 'h' || k === 'H') { panel.hidden = !panel.hidden; return; }
    if (k === 'r' || k === 'R') { reiniciar(); return; }
    if (k === 'f' || k === 'F') {
      if (document.fullscreenElement) document.exitFullscreen();
      else document.documentElement.requestFullscreen().then(() => navigator.keyboard?.lock?.(['Escape'])).catch(() => {});
      return;
    }
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Enter', 'Escape'].includes(k)) return;
    ev.preventDefault();
    if (st.arranque || st.reloj < .3) return;
    st.cuentaActiva = false;
    const m = st.menu;
    if (m.abierto) {
      if (k === 'ArrowUp') m.i = (m.i + OPCIONES.length - 1) % OPCIONES.length;
      else if (k === 'ArrowDown') m.i = (m.i + 1) % OPCIONES.length;
      else if (k === 'Enter') avisar(OPCIONES[m.i].aviso);
      else if (k === 'Escape') m.abierto = false;
      return;
    }
    const orden = ORDENES[cfg.orden], i = orden.indexOf(st.elegido);
    if (k === 'ArrowLeft') st.elegido = orden[Math.max(0, i - 1)];
    else if (k === 'ArrowRight') st.elegido = orden[Math.min(2, i + 1)];
    else if (k === 'Enter') arrancar(st.elegido);
    else if (k === 'Escape') Object.assign(m, { abierto: true, i: 0 });
  });

  const panel = document.getElementById('panel');
  function marcarPanel() {
    for (const b of panel.querySelectorAll('button[data-k]')) b.classList.toggle('activo', cfg[b.dataset.k] === b.dataset.v);
  }
  panel.addEventListener('click', (ev) => {
    const b = ev.target.closest('button[data-k]');
    if (!b) return;
    b.blur();
    cfg[b.dataset.k] = b.dataset.v;
    if (cfg.fondo === 'sube' && !subiendo.length) for (let i = 0; i < 8; i++) { const p = humoNuevo(azar(0, 14)); p.y += p.vy * p.t; subiendo.push(p); }
    marcarPanel();
  });

  // ================================================================ arranque de la maqueta
  reiniciar();
  marcarPanel();
  let previo = 0;
  (function fotograma(ahora) {
    const dt = Math.min(.05, previo ? (ahora - previo) / 1000 : 1 / 60);
    previo = ahora;
    actualizar(dt);
    pintar();
    requestAnimationFrame(fotograma);
  })(0);
})();

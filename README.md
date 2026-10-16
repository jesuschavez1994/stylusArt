# StylusArt

Editor de imágenes y dibujo para **Android**, pensado para la **Xiaomi Pad 7** con el
**Xiaomi Focus Pen**. Es de uso personal y se instala directamente, sin Play Store.

StylusArt es un **fork de [PhotoCraft](https://github.com/storytold/photocraft)**, el editor open
source escrito en Rust (egui + winit + wgpu). Todo el motor (capas, máscaras, PSD, pinceles y
filtros) viene de PhotoCraft. Este fork agrega la app de Android y una interfaz táctil propia para
dibujar con lápiz. El README original de PhotoCraft está en
[`docs/PHOTOCRAFT_README.md`](docs/PHOTOCRAFT_README.md).

> StylusArt no está afiliado a ArtCraft ni a PhotoCraft. Los nombres y logos de ArtCraft y
> PhotoCraft son marcas de sus dueños y no forman parte de este proyecto.

## Qué puedes hacer

Todo se usa desde la interfaz táctil de la app: botones grandes para dibujar con lápiz y dedos.

### Dibujar
- **Lápiz con presión**: el ancho del trazo responde a la presión del Focus Pen (se puede desactivar).
- **Rechazo de palma**: sobre el lienzo solo pinta el lápiz; la mano apoyada no raya. Los dedos
  siguen funcionando en botones y paneles. Menú › *Dibujar con el dedo* lo desactiva.
- **Gestos con dos dedos**: mover, hacer zoom y rotar el lienzo. **Toque con dos dedos** deshace,
  **con tres dedos** rehace.
- **Panel del pincel** (toca otra vez la herramienta activa, o el botón de ajustes): vista previa
  real del trazo, ancho, opacidad, dureza y presión.
- **Biblioteca de pinceles** con el trazo real de cada uno y buscador.
- **Simetría**: vertical, horizontal, doble o diagonal.
- **Selector de color táctil**: cuadro de saturación/brillo, barra de tono, color anterior y nuevo,
  colores recientes y muestras.

### Herramientas
Pincel, lápiz, borrador, difuminar, bote de pintura, degradado, cuentagotas, **texto**, lazo,
marco rectangular, mover, **recortar**, **parche** y **mesa de trabajo**, más el botón
**Transformar** (transformación libre, voltear, rotar 90°). Mientras haya un recorte, una
transformación o un texto en curso aparece la barra **Cancelar / Aplicar**.

### Texto
- Teclado en pantalla automático al escribir.
- Tamaño, alineación, color y **tipografías**: las del sistema (MiSans, Roboto, Noto…) y 20 fuentes
  libres incluidas en la app (Montserrat, Poppins, Bebas Neue, Lobster, Pacifico, Playfair Display,
  Caveat, Permanent Marker, entre otras), con buscador.

### Capas
- Nueva, duplicar, eliminar, visibilidad, opacidad, grupos y mesas de trabajo anidadas.
- **Quitar fondo**: selecciona el sujeto y crea una máscara de capa (el fondo queda oculto, no
  borrado, y se puede retocar pintando la máscara).
- **Máscara de capa** y **máscara de recorte**.
- **Modos de fusión**: Multiplicar, Trama, Superponer, Luz suave, Luz fuerte, Sobreexponer y
  Subexponer color, Oscurecer, Aclarar, Diferencia, Tono, Saturación, Color, Luminosidad…

### Archivos
- **Abrir imagen** desde una galería con miniaturas (Imágenes, Cámara, Capturas y Descargas):
  PNG, JPG, WebP, PSD y archivos de StylusArt (`.pcraft`).
- **Exportar** a *Imágenes/StylusArt*: PNG, **PNG sin fondo** (sin la capa Fondo) y JPG.
- Mesas de trabajo de tamaños predefinidos (cuadrado, historia, horizontal, A4).

## Novedades por versión

| Versión | Novedades |
|---|---|
| **v6** | Gestos (pellizcar para zoom, rotar, mover; dos y tres dedos para deshacer y rehacer); biblioteca de pinceles con trazo real; simetría; modos de fusión; botón Transformar; vista previa del pincel hecha con el motor de pinceles. |
| **v5** | Fluidez: la pantalla se actualiza al instante tras cada toque (antes parecía que se quedaba pegada al cambiar de capa o de color); toques más tolerantes; nuevo selector de color táctil con colores recientes; tocar el lienzo cierra los paneles. |
| **v4** | Herramienta de texto con teclado y 20 tipografías; quitar fondo; máscaras de capa y de recorte; recortar, parche y mesas de trabajo; barra Cancelar/Aplicar; exportar PNG (con y sin fondo) y JPG; galería para abrir fotos. |
| **v3** | Rechazo de palma (el lápiz pinta, la mano no); panel del pincel con vista previa, ancho, opacidad, dureza y presión. |
| **v2** | Interfaz táctil estilo tablet: barra superior, columna de herramientas grande, panel de capas, selector de color. |
| **v1** | Primera APK: el editor de PhotoCraft funcionando en la Pad 7 con aceleración por GPU (Vulkan) y presión del lápiz. |

## Estado

| Fase | Qué incluye | Estado |
|---|---|---|
| 0 | Viabilidad del port | hecho |
| 1 | APK que abre el editor y pinta con presión | hecho |
| 2 | Lápiz: rechazo de palma, gestos | hecho (falta hover, inclinación, botones del lápiz y puntos intermedios para más suavidad) |
| 3 | Interfaz para tablet | hecho, en mejora continua |
| 4 | Archivos: abrir y exportar | hecho (falta autoguardado y guardar en formato propio/PSD) |
| 5 | APK firmada con keystore personal | hecho |

**Próximas ideas:** llevar a la interfaz táctil los ajustes de imagen (curvas, niveles,
tono/saturación…), los filtros (desenfoque gaussiano y otros), los estilos de capa y el historial
visual; integración con IA para mejorar luz y fondos.

## Compilar la APK (Windows)

Requisitos: Rust con el target `aarch64-linux-android`, Android SDK + NDK (r26), JDK 17 y
`cargo-apk`.

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk --locked

# Tipografías incluidas (no están en el repositorio): se descargan una vez.
sh apps/stylusart-android/fetch-fonts.sh

export ANDROID_HOME=<ruta al Android SDK>
export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/<versión>
# Firma con tu keystore personal (guárdalo fuera del repositorio).
export CARGO_APK_RELEASE_KEYSTORE=<ruta al .keystore>
export CARGO_APK_RELEASE_KEYSTORE_PASSWORD=<contraseña>

cargo apk build --release -p stylusart-android
```

La APK queda en `target/release/apk/StylusArt.apk` (o en `$CARGO_TARGET_DIR/release/apk/`).
Usa siempre el mismo keystore: si cambia la firma, Android obliga a desinstalar la app antes de
instalar la nueva versión.

## Instalar en la tablet

**Copiando el archivo:** pasa `StylusArt.apk` a la tablet (por ejemplo a *Descargas*), ábrelo
desde el gestor de archivos y acepta "Instalar apps desconocidas".

**Por USB con adb:**
1. Ajustes → Acerca de la tableta → toca "Versión del SO" 7 veces.
2. En Opciones de desarrollador, activa "Depuración USB" e "Instalar vía USB".
3. Conecta la tablet, acepta el permiso y ejecuta `adb install -r StylusArt.apk`.

La primera vez que abras la galería, la app pide permiso para ver tus fotos.

## Código

- `apps/stylusart-android`: la app de Android (NativeActivity + eframe): teclado, tipografías,
  permisos, exportación y la conexión de los toques con el rechazo de palma y los gestos.
- `crates/ui-egui/src/touch_ui.rs`: toda la interfaz táctil (herramientas, paneles, color, capas,
  galería, gestos).
- `vendor/winit`: winit 0.30.13 con un parche pequeño que entrega el tipo de cada toque (lápiz,
  dedo, borrador) y su tamaño; solo cambian `src/platform/android.rs` y
  `src/platform_impl/android/mod.rs`.
- `crates/ui-egui/examples/snapshot.rs --touch`: renderiza la interfaz táctil a PNG en la PC;
  `examples/touch_bench.rs` mide la respuesta al cambiar capas y colores.
- Todo lo demás es PhotoCraft; ver [`AGENTS.md`](AGENTS.md) y
  [`docs/architecture.md`](docs/architecture.md).
- Remotes: `origin` = este repo; `upstream` = `storytold/photocraft`, para traer mejoras.

## Licencia

Igual que PhotoCraft: **MIT o Apache-2.0**, a tu elección ([`LICENSE-MIT`](LICENSE-MIT),
[`LICENSE-APACHE`](LICENSE-APACHE)). Copyright (c) 2026 ArtCraft Team y los contribuidores de
PhotoCraft; los cambios de StylusArt se publican bajo las mismas licencias. Los recursos de
terceros conservan sus licencias ([`ATTRIBUTION.md`](ATTRIBUTION.md)); winit (`vendor/winit`)
es Apache-2.0, y las tipografías incluidas en la APK son SIL Open Font License (Permanent Marker,
Apache-2.0), con su licencia junto a cada una.

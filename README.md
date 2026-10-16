# StylusArt

Editor de imágenes y dibujo para **Android**, pensado para la **Xiaomi Pad 7** con el
**Xiaomi Focus Pen**. Es de uso personal y se instala directamente, sin Play Store.

StylusArt es un **fork de [PhotoCraft](https://github.com/storytold/photocraft)**, el editor open
source escrito en Rust (egui + winit + wgpu). Todo el motor (capas, máscaras, PSD, pinceles y
filtros) viene de PhotoCraft. Este fork agrega la app de Android y la adaptación para tablet con
lápiz. El README original de PhotoCraft está en
[`docs/PHOTOCRAFT_README.md`](docs/PHOTOCRAFT_README.md).

> StylusArt no está afiliado a ArtCraft ni a PhotoCraft. Los nombres y logos de ArtCraft y
> PhotoCraft son marcas de sus dueños y no forman parte de este proyecto.

## Estado

| Fase | Qué incluye | Estado |
|---|---|---|
| 0 | Viabilidad del port | hecho |
| 1 | APK que abre el editor y pinta con dedo y lápiz (presión vía winit) | en curso |
| 2 | Lápiz completo: tipo de herramienta, rechazo de palma, hover, inclinación, botones, puntos históricos, baja latencia | pendiente |
| 3 | Interfaz para tablet (escalado, controles táctiles, gestos) | pendiente |
| 4 | Archivos (Storage Access Framework), autoguardado | pendiente |
| 5 | APK release con keystore personal | pendiente |

## Compilar la APK (Windows)

Requisitos: Rust con el target `aarch64-linux-android`, Android SDK + NDK (r26), JDK 17 y
`cargo-apk`.

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk --locked
export ANDROID_HOME=<ruta al Android SDK>
export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/<versión>
cargo apk build --release -p stylusart-android
```

La APK queda en `target/release/apk/StylusArt.apk` (o en `$CARGO_TARGET_DIR/release/apk/`).

## Instalar en la tablet

1. En la tablet, ve a Ajustes → Acerca de la tableta y toca "Versión del SO" 7 veces.
2. En Opciones de desarrollador, activa "Depuración USB" e "Instalar vía USB".
3. Conecta la tablet por USB y ejecuta `adb install -r StylusArt.apk`.

## Código

- `apps/stylusart-android`: la app de Android (NativeActivity + eframe).
- Todo lo demás es PhotoCraft; ver [`AGENTS.md`](AGENTS.md) y [`docs/architecture.md`](docs/architecture.md).
- Remotes: `origin` = este repo; `upstream` = `storytold/photocraft`, para traer mejoras.

## Licencia

Igual que PhotoCraft: **MIT o Apache-2.0**, a tu elección ([`LICENSE-MIT`](LICENSE-MIT),
[`LICENSE-APACHE`](LICENSE-APACHE)). Copyright (c) 2026 ArtCraft Team y los contribuidores de
PhotoCraft; los cambios de StylusArt se publican bajo las mismas licencias. Los recursos de
terceros conservan sus licencias ([`ATTRIBUTION.md`](ATTRIBUTION.md)).

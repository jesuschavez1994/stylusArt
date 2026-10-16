# Todas las funciones de StylusArt

StylusArt incluye el motor completo de PhotoCraft: **632 comandos de menú** y **67 herramientas**, todos implementados. Esta lista se genera desde el código (`crates/ui-egui/src/menu_catalog.rs` y los nombres en español de `i18n/es.tsv`).

**Dónde está cada función:**

- ✋ = tiene botón propio en la **interfaz táctil**.
- Todo lo demás se usa desde **Menú (…) → Interfaz de escritorio**, que muestra los menús completos (Archivo, Edición, Imagen, Capa, Texto, Selección, Filtro, Vista, Ventana) y el panel de herramientas completo. Para volver a la interfaz táctil, cierra y abre la app.

Con botón táctil: 20 comandos y 14 herramientas.

## Herramientas

- Herramienta Mover ✋
- Herramienta Lienzo ✋
- Herramienta Marco rectangular ✋
- Herramienta Marco elíptico
- Herramienta Marco de fila única
- Herramienta Marco de columna única
- Herramienta Pincel ✋
- Herramienta Lápiz ✋
- Herramienta Pincel mezclador
- Herramienta Sustitución de color
- Herramienta Borrador ✋
- Herramienta Borrador de fondos
- Herramienta Borrador mágico
- Herramienta Cuentagotas ✋
- Color Sampler Tool
- Herramienta Regla
- Herramienta Nota
- Herramienta Recuento
- Herramienta Lazo ✋
- Herramienta Lazo poligonal
- Herramienta Lazo magnético
- Herramienta Varita mágica
- Herramienta Recortar ✋
- Herramienta Recortar con perspectiva
- Herramienta Sector
- Herramienta Selección de sector
- Herramienta Marco
- Herramienta Degradado ✋
- Herramienta Bote de pintura ✋
- Herramienta Texto horizontal ✋
- Herramienta Texto vertical
- Herramienta Máscara de texto horizontal
- Herramienta Máscara de texto vertical
- Herramienta Mano
- Herramienta Rotar vista
- Herramienta Zoom
- Herramienta Quitar
- Herramienta Pincel corrector puntual
- Herramienta Pincel corrector
- Herramienta Parche ✋
- Herramienta Mover con detección de contenido
- Herramienta Ojos rojos
- Herramienta Tampón de clonar
- Herramienta Tampón de motivo
- Herramienta Pincel de historia
- Herramienta Desenfocar
- Herramienta Enfocar
- Herramienta Dedo ✋
- Herramienta Sobreexponer
- Herramienta Subexponer
- Herramienta Esponja
- Herramienta Selección rápida
- Herramienta Selección de objetos
- Herramienta Pluma
- Herramienta Pluma de forma libre
- Herramienta Pluma de curvatura
- Herramienta Añadir punto de ancla
- Herramienta Eliminar punto de ancla
- Herramienta Convertir punto
- Herramienta Selección de trazado
- Herramienta Selección directa
- Herramienta Rectángulo
- Herramienta Elipse
- Herramienta Triángulo
- Herramienta Polígono
- Herramienta Línea
- Herramienta Forma personalizada

## Archivo

- Nuevo… ✋
- Abrir… ✋
- Abrir como…
- Abrir partes EXR…
- Cerrar
- Cerrar todo
- Cerrar otros
- Guardar
- Guardar como…
- Guardar una copia…
- Volver

**Exportar**

- Exportación rápida como PNG
- Exportar como… ✋
- Preferencias de exportación…
- Guardar para web (heredado)…
- Mesas de trabajo en archivos…
- Mesas de trabajo en PDF…
- Capas en archivos…
- Composiciones de capa en archivos…
- Tablas de consulta de colores…
- Conjuntos de datos en archivos…
- Trazados a Illustrator…
- Procesar vídeo…

**Generar**

- Recursos de imagen
- Colocar incrustado…
- Colocar vinculado…
- Empaquetar…

**Automatizar**

- Lote…
- Crear droplet…
- Recortar y enderezar fotos
- Hoja de contactos II…
- Cambio de modo condicional…
- Ajustar imagen…
- Corrección de lente…
- Combinar en HDR Pro…
- Photomerge…

**Secuencias de comandos**

- Procesador de imágenes…
- Eliminar todas las capas vacías
- Acoplar todos los efectos de capa
- Acoplar todas las máscaras
- Administrador de eventos de secuencias de comandos…
- Cargar archivos en pila…
- Estadísticas…
- Examinar…

**Importar**

- Conjuntos de datos variables…
- Fotogramas de vídeo a capas…
- Notas…
- Compatibilidad con WIA…
- Información del archivo…
- Imprimir…
- Imprimir una copia
- Salir

## Edición

- Deshacer
- Rehacer
- Alternar último estado
- Transición…
- Cortar
- Copiar
- Copiar combinado
- Pegar

**Pegado especial**

- Pegar en contexto
- Pegar dentro
- Pegar fuera
- Borrar
- Buscar…
- Revisar ortografía…
- Buscar y reemplazar texto…
- Rellenar…
- Contornear…
- Relleno según el contenido…
- Escala según el contenido
- Deformación de posición libre
- Deformación de perspectiva
- Transformación libre ✋

**Transformar**

- Repetir
- Escalar
- Rotar
- Sesgar
- Distorsionar
- Perspectiva
- Deformar
- Dividir deformación horizontalmente
- Dividir deformación verticalmente
- Dividir deformación en cruz
- Quitar división de deformación
- Rotar 180°
- Rotar 90° en sentido horario ✋
- Rotar 90° en sentido antihorario ✋
- Voltear horizontal ✋
- Voltear vertical ✋
- Alinear capas automáticamente…
- Fusionar capas automáticamente…
- Definir ajuste preestablecido de pincel…
- Definir motivo…
- Definir forma personalizada…

**Purgar**

- Deshacer
- Portapapeles
- Historias
- Caché de vídeo
- Todo

**Ajustes preestablecidos**

- Gestor de ajustes preestablecidos…
- Migrar ajustes preestablecidos
- Exportar/importar ajustes preestablecidos…
- Ajustes de color…
- Asignar perfil…
- Convertir a perfil…
- Métodos abreviados de teclado…
- Menús…
- Barra de herramientas…

**Preferencias**

- Configuración…
- Integraciones de IA…
- Interfaz…
- Espacio de trabajo…
- Herramientas…
- Registro de historia…
- Gestión de archivos…
- Exportación…
- Rendimiento…
- Discos de memoria virtual…
- Cursores…
- Transparencia y gama…
- Unidades y reglas…
- Guías, cuadrícula y sectores…
- Plugins…
- Texto…
- Controles mejorados…
- Valores predeterminados de Raw…

## Imagen


**Modo**

- Mapa de bits
- Escala de grises
- Duotono
- Color indexado
- Color RGB
- Color CMYK
- Color Lab
- Multicanal
- 8 bits/canal
- 16 bits/canal
- 32 bits/canal
- Tabla de colores…

**Ajustes**

- Brillo/contraste…
- Niveles…
- Curvas…
- Exposición…
- Intensidad…
- Tono/saturación…
- Equilibrio de color…
- Blanco y negro…
- Filtro de fotografía…
- Mezclador de canales…
- Consulta de colores…
- Invertir
- Posterizar…
- Umbral…
- Mapa de degradado…
- Corrección selectiva…
- Sombras/iluminaciones…
- Tonificación HDR…
- Desaturar
- Igualar color…
- Reemplazar color…
- Ecualizar
- Tono automático
- Contraste automático
- Color automático
- Tamaño de imagen…
- Tamaño de lienzo…

**Rotación de imagen**

- 180°
- 90° en sentido horario
- 90° en sentido antihorario
- Arbitrario…
- Voltear el lienzo horizontalmente
- Voltear el lienzo verticalmente
- Recortar
- Cortar… ✋
- Descubrir todo
- Duplicar…
- Aplicar imagen…
- Cálculos…

**Variables**

- Definir…
- Conjuntos de datos…
- Aplicar conjunto de datos…
- Reventado…

**Análisis**

- Definir escala de medida…
- Seleccionar puntos de datos…
- Registrar medidas
- Herramienta Regla
- Herramienta Recuento
- Colocar marcador de escala…

## Capa


**Nuevo**

- Capa… ✋
- Capa a partir del fondo… ✋
- Fondo a partir de capa
- Grupo…
- Grupo a partir de capas…
- Mesa de trabajo… ✋
- Mesa de trabajo a partir de grupo…
- Mesa de trabajo a partir de capas…
- Fotograma a partir de capas
- Capa vía copiar
- Capa vía cortar
- Duplicar capa… ✋

**Eliminar**

- Capa ✋
- Capas ocultas
- Exportación rápida como PNG
- Exportar como…
- Cambiar nombre de capa

**Estilo de capa**

- Opciones de fusión…
- Bisel y relieve…
- Contornear…
- Sombra interior…
- Resplandor interior…
- Satinado…
- Superposición de color…
- Superposición de degradado…
- Superposición de motivo…
- Resplandor exterior…
- Sombra paralela…
- Copiar estilo de capa
- Pegar estilo de capa
- Borrar estilo de capa
- Luz global…
- Crear capa
- Ocultar todos los efectos
- Escalar efectos…

**Filtro inteligente**

- Desactivar filtros inteligentes
- Eliminar máscara de filtro
- Desactivar máscara de filtro
- Opciones de fusión…
- Borrar filtros inteligentes

**Nueva capa de relleno**

- Color sólido…
- Degradado…
- Motivo…

**Nueva capa de ajuste**

- Brillo/contraste…
- Niveles…
- Curvas…
- Exposición…
- Intensidad…
- Tono/saturación…
- Equilibrio de color…
- Blanco y negro…
- Filtro de fotografía…
- Mezclador de canales…
- Consulta de colores…
- Invertir…
- Posterizar…
- Umbral…
- Mapa de degradado…
- Corrección selectiva…
- Opciones de contenido de capa…

**Máscara de capa**

- Descubrir todo ✋
- Ocultar todo
- Descubrir selección ✋
- Ocultar selección
- Desde transparencia
- Eliminar ✋
- Aplicar
- Activar máscara de capa
- Vincular máscara de capa
- Enmascarar todos los objetos

**Máscara vectorial**

- Descubrir todo
- Ocultar todo
- Trazado actual
- Eliminar
- Activar máscara vectorial
- Vincular máscara vectorial
- Crear máscara de recorte

**Objetos inteligentes**

- Convertir en objeto inteligente
- Nuevo objeto inteligente mediante copia
- Mostrar en Finder
- Actualizar contenido modificado
- Actualizar todo el contenido modificado
- Editar contenido
- Volver a vincular al archivo…
- Reemplazar contenido…
- Exportar contenido…
- Convertir en vinculado…
- Convertir en incrustado
- Convertir en capas

**Objetos inteligentes › Modo de apilamiento**

- Entropía
- Curtosis
- Máximo
- Media
- Mediana
- Mínimo
- Intervalo
- Asimetría
- Desviación estándar
- Sumatoria
- Varianza

**Objetos inteligentes**

- Rasterizar
- Deformar
- Deformación de perspectiva
- Deformación de posición libre

**Capas de vídeo**

- Nueva capa de vídeo en blanco
- Nueva capa de vídeo desde archivo…
- Insertar fotograma en blanco
- Duplicar fotograma
- Eliminar fotograma
- Reemplazar material de archivo…
- Interpretar material de archivo…
- Mostrar vídeo alterado
- Restaurar fotograma
- Restaurar todos los fotogramas
- Recargar fotograma
- Rasterizar

**Rasterizar**

- Texto
- Forma
- Contenido de relleno
- Máscara vectorial
- Objeto inteligente
- Vídeo
- Estilo de capa
- Capa
- Todas las capas
- Nuevo sector basado en capa
- Agrupar capas
- Desagrupar capas
- Ocultar capas

**Organizar**

- Traer al frente
- Traer adelante
- Enviar atrás
- Enviar al fondo
- Invertir

**Combinar formas**

- Unir formas
- Restar la forma frontal
- Intersecar áreas de formas
- Excluir formas superpuestas
- Combinar componentes de formas

**Alinear**

- Bordes superiores
- Centros verticales
- Bordes inferiores
- Bordes izquierdos
- Centros horizontales
- Bordes derechos

**Distribuir**

- Bordes superiores
- Centros verticales
- Bordes inferiores
- Bordes izquierdos
- Centros horizontales
- Bordes derechos
- Horizontalmente
- Verticalmente
- Bloquear capas…
- Vincular capas
- Seleccionar capas vinculadas
- Combinar capas
- Combinar visibles
- Acoplar imagen

**Halos**

- Descontaminar color…
- Eliminar halos…
- Eliminar halo negro
- Eliminar halo blanco

## Texto


**Paneles**

- Panel Carácter
- Panel Párrafo
- Panel Glifos
- Panel Estilos de carácter
- Panel Estilos de párrafo

**Suavizado**

- Ninguno
- Enfocado
- Nítido
- Fuerte
- Suave
- LCD de Windows
- Windows

**Orientación**

- Horizontal
- Vertical

**OpenType**

- Ligaduras estándar
- Alternativas contextuales
- Ligaduras discrecionales
- Caracteres decorativos
- Estilo antiguo
- Alternativas estilísticas
- Alternativas para titulares
- Ornamentos
- Ordinales
- Fracciones
- Crear trazado de trabajo
- Convertir en forma
- Rasterizar capa de texto
- Convertir en texto de párrafo
- Convertir en texto de punto
- Deformar texto…

**Tamaño de vista previa de fuentes**

- Pequeño
- Mediano
- Grande
- Extra grande
- Enorme

**Opciones de idioma**

- Funciones predeterminadas
- Funciones de Asia oriental
- Funciones de Oriente Medio
- Compositor de Oriente Medio y Asia meridional
- Actualizar todas las capas de texto
- Reemplazar todas las fuentes que faltan
- Resolver fuentes que faltan…
- Pegar Lorem Ipsum
- Cargar estilos de texto predeterminados
- Guardar estilos de texto predeterminados

## Selección

- Todo
- Deseleccionar ✋
- Volver a seleccionar
- Invertir selección
- Convertir en forma
- Todas las capas
- Deseleccionar capas
- Buscar capas
- Aislar capas
- Gama de colores…
- Área de enfoque…
- Sujeto ✋
- Cielo
- Selección y máscara…

**Modificar**

- Borde…
- Suavizar…
- Expandir…
- Contraer…
- Calar…
- Extender
- Similar
- Transformar selección
- Editar en modo de máscara rápida
- Cargar selección…
- Guardar selección…

## Filtro

- Último filtro
- Convertir para filtros inteligentes
- Galería de filtros…
- Filtro de Camera Raw…
- Gran angular adaptable…
- Corrección de lente…
- Licuar…
- Punto de fuga…

**Desenfocar**

- Promedio
- Desenfocar
- Desenfocar más
- Desenfoque de cuadro…
- Desenfoque gaussiano…
- Desenfoque de lente…
- Desenfoque de movimiento…
- Desenfoque radial…
- Desenfoque de forma…
- Desenfoque inteligente…
- Desenfoque de superficie…

**Galería de desenfoque**

- Desenfoque de campo…
- Desenfoque de iris…
- Cambio de inclinación…
- Desenfoque de trazado…
- Desenfoque de rotación…

**Distorsionar**

- Desplazar…
- Pellizcar…
- Coordenadas polares…
- Ondulación…
- Inclinación…
- Esferizar…
- Remolino…
- Onda…
- Zigzag…

**Ruido**

- Añadir ruido…
- Destramar
- Polvo y arañazos…
- Mediana…
- Reducir ruido…

**Pixelizar**

- Semitono de color…
- Cristalizar…
- Faceta
- Fragmento
- Grabado…
- Mosaico…
- Puntillista…

**Interpretar**

- Llama…
- Marco…
- Árbol…
- Nubes
- Nubes de diferencia
- Fibras…
- Destello…
- Efectos de iluminación…

**Enfocar**

- Enfocar
- Enfocar bordes
- Enfocar más
- Enfoque suavizado…
- Máscara de enfoque…

**Estilizar**

- Difundir…
- Relieve…
- Extruir…
- Hallar bordes
- Pintura al óleo…
- Solarizar
- Azulejos…
- Trazar contorno…
- Viento…

**Vídeo**

- Desentrelazar…
- Colores NTSC

**Otro…**

- Personalizado…
- HSB/HSL
- Paso alto…
- Máximo…
- Mínimo…
- Desplazamiento…

## Vista


**Ajuste de prueba**

- Personalizado…
- CMYK de trabajo
- Placa cian de trabajo
- Placa magenta de trabajo
- Placa amarilla de trabajo
- Placa negra de trabajo
- Placa CMY de trabajo
- RGB heredado de Macintosh
- RGB estándar de Internet
- RGB del monitor
- Daltonismo — tipo protanopia
- Daltonismo — tipo deuteranopia
- Colores de prueba
- Advertencia de gama

**Relación de aspecto de píxel**

- Relación de aspecto de píxel personalizada…
- Cuadrado
- D1/DV NTSC (0.91)
- D1/DV PAL (1.09)
- D1/DV NTSC panorámico (1.21)
- HDV 1080/DVCPRO HD 720 (1.33)
- D1/DV PAL panorámico (1.46)
- DVCPRO HD 1080/HDV 1080 (1.5)
- Anamórfico 2:1 (2)
- Corrección de la relación de aspecto de píxel
- Opciones de vista previa de 32 bits…
- Acercar
- Alejar
- Encajar en pantalla ✋
- Encajar capas en pantalla
- Encajar mesa de trabajo en pantalla
- 100%
- 200%
- Tamaño de impresión
- Voltear horizontal

**Modo de pantalla**

- Modo de pantalla estándar
- Modo de pantalla completa con barra de menús
- Modo de pantalla completa
- Extras

**Mostrar**

- Bordes de capa
- Bordes de la selección
- Trazado de destino
- Cuadrícula
- Guías
- Guías del lienzo
- Guías de la mesa de trabajo
- Recuento
- Guías inteligentes
- Sectores
- Notas
- Cuadrícula de píxeles
- Vista previa del pincel
- Malla
- Editar chinchetas
- Todo
- Opciones de extras…
- Reglas
- Ajustar

**Ajustar a**

- Guías
- Cuadrícula
- Capas
- Sectores
- Límites del documento
- Todo
- Bloquear guías
- Borrar guías
- Borrar guías de la mesa de trabajo seleccionada
- Borrar guías del lienzo
- Nueva guía…
- Nuevo diseño de guías…
- Guías nuevas a partir de la forma
- Bloquear sectores
- Borrar sectores
- Vista previa de motivo
- Vista previa de arte de píxeles

## Ventana


**Organizar**

- Mosaico vertical de todas
- Mosaico horizontal de todas
- 2 ventanas (vertical)
- 2 ventanas (horizontal)
- 3 ventanas (vertical)
- 3 ventanas (horizontal)
- 3 ventanas (apiladas)
- 4 ventanas
- 6 ventanas
- Consolidar todo en pestañas
- Cascada
- Mosaico
- Hacer flotante en ventana
- Hacer flotantes todas en ventanas
- Nueva ventana para el documento
- Igualar zoom
- Igualar ubicación
- Igualar rotación
- Igualar todo

**Espacio de trabajo**

- Esenciales
- Fotografía
- Pintura
- Arte de píxeles
- Gráficos y web
- Movimiento
- Restablecer espacio de trabajo
- Nuevo espacio de trabajo…
- Eliminar espacio de trabajo…
- Métodos abreviados de teclado…
- Bloquear espacio de trabajo
- Acciones
- Ajustes
- Ajustes del pincel
- Pinceles
- Canales
- Carácter
- Estilos de carácter
- Origen de clonado
- Color
- Glifos
- Degradados
- Histograma
- Historia
- Información
- Composiciones de capa
- Capas
- Registro de medidas
- Teclas modificadoras
- Navegador
- Notas
- Párrafo
- Estilos de párrafo
- Trazados
- Motivos
- Propiedades
- Formas
- Estilos
- Muestras
- Línea de tiempo
- Ajustes preestablecidos de herramienta
- Opciones
- Herramientas


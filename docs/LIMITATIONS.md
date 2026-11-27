# Limitaciones conocidas y soluciones

| Limitación | Por qué | Solución actual |
|---|---|---|
| MeshPart/Unions se ven como caja en alambre | El archivo solo guarda el `MeshId`; la malla está en la nube de Roblox | Tamaño y posición reales; para ver la malla exporta el modelo desde Studio como .fbx/.obj y súbelo como versión |
| .gltf con archivos externos | El visor recibe un solo archivo | Importar como .glb |
| Reproducción de animaciones | Requiere un rig y los datos de las poses | Se extraen keyframes, duración, bucle y articulaciones; la reproducción queda para una versión futura |
| Búsqueda "semántica" | Sin modelos de IA ni servicios externos | Ranking BM25 local con camelCase, prefijos y código |
| SmartScreen al instalar | El instalador no tiene certificado de pago | "Más información → Ejecutar de todas formas" |
| El formato .rbxm no tiene contrato estable | Lo dice la documentación de Roblox | Se usan los lectores de Rojo (rbx_binary/rbx_xml) y SerializationService |

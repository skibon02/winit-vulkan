# Changelog
A place where every action with the project is tracked.

## TODO

## Planned
### Renderer abstraction
🔨 Remove direct vulkan dependency and support OpenGL renderer \
🔨 Support WebGL renderer for web browser platform

### App logic
🔨 Prepare simple manual app logic abstraction \
🔨 Support WASM platform in app logic

### Platforms
🔨 Implement OpenXR support as alternative to Winit platform \
🔨 Support WASM platform in renderer \
🔨 Make renderer and app work in browser (require webgl and WASM support)

## In progress
### Milestone: **Simple 2d app**
⚙️ Resource loading abstraction \
⚙️ Uniform abstraction (image sampler)


## Done
✅ Attribute fields diff support \
✅ Uniform abstraction (only buffer) \
✅ Pipeline abstraction \
✅ Render state abstraction \
✅ Beging generalizing renderer \
✅ Separate application and renderer \
✅ Basic use of image samplers \
✅ Remove manual destroy/free calls, use wrappers with RAII. \
✅ Implement basic vertex buffer interaction \
✅ Implement basic uniform buffer interaction \
✅ Implement swapchain recreation on resize \
✅ Draw a triangle \
✅ Draw solid color \
✅ Basic vulkan initialization
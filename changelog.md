# Changelog
A place where every action with the project is tracked.

## Planned
### Renderer abstraction
🔨 Remove direct vulkan dependency and support OpenGL render \
🔨 Support WebGL render for web browser platform

### App logic
🔨 Prepare simple manual app logic abstraction \
🔨 Support WASM platform in app logic

### Platforms
🔨 Implement OpenXR support as alternative to Winit platform \
🔨 Support WASM platform in render \
🔨 Make render and app work in browser (require webgl and WASM support)

## In progress
### Milestone: **Simple 2d app**
⚙️ Implement multiple instances object


## Done
### 03.12.2024
✅ define_layout! and #[derive(CollectDrawStateUpdates)] \
✅ Separate to core, derive and render and app crates \
✅ Introduced resource update abstraction: new/update/delete
### Earlier
✅ Uniform abstraction (image sampler) \
✅ Attribute fields diff support \
✅ Uniform abstraction (only buffer) \
✅ Pipeline abstraction \
✅ Render state abstraction \
✅ Beging generalizing render \
✅ Separate application and render \
✅ Basic use of image samplers \
✅ Remove manual destroy/free calls, use wrappers with RAII. \
✅ Implement basic vertex buffer interaction \
✅ Implement basic uniform buffer interaction \
✅ Implement swapchain recreation on resize \
✅ Draw a triangle \
✅ Draw solid color \
✅ Basic vulkan initialization
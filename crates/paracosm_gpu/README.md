# Paracosm GPU

A frame graph based GPU-driven Vulkan abstraction library.

## 1.0.0 Release development checklist

- Command recording
  - [ ] Common commands
  - [ ] Compute commands
  - [ ] Graphics commands
  - [ ] Transfer commands

- Resource management
  - [ ] Buffers
    - [x] Persistent
    - [ ] Transient
  - [ ] Images
    - [x] Persistent
    - [ ] Transient
  - [ ] Acceleration Structures
  - [ ] Automatic host-device resource transfers

- Shader integration
  - [x] Type sharing between host and shader code
  - [x] Typed push constants

- Synchronization
  - [ ] Automatic memory barriers
  - [ ] Automatic image layout transitions
  - [ ] Explicit cross-queue sync

- WSI and swapchain management
  - [x] Acquire and present
  - [ ] Window resize and minimize
  - [ ] Multiwindow

- Usage examples and samples
  - [x] Hello Compute
  - [x] Hello Triangle
  - [x] Game of Life
  - [ ] Boids
  - [ ] Fluid Sim

## Frame Graph features analysis

High priority

- Automatic resource sync (layout transitions and memory barriers)
- Transient resource aliasing
- Automatic resource lifetime management
- Automatic resource host-device transfers

Low priority

- Multithreaded command recording

Ignored

- Automatic node ordering for resource dependencies

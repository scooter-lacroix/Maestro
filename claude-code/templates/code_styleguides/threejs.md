# Three.js Guide

When I write Three.js, I treat rendering, interaction, and asset lifecycle as separate concerns so the scene stays maintainable as it grows.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Stable frame time and deliberate resource management.
- Scene structure that matches the product's mental model.
- Predictable interaction handling and camera behavior.
- A rendering layer that can evolve without swallowing the whole app.

## Required defaults

- Keep world units, coordinate conventions, and camera assumptions consistent across the project.
- Separate scene setup, asset loading, interaction logic, and the animation loop into distinct modules.
- Dispose of geometries, materials, textures, and listeners when objects leave the scene.
- Minimize per-frame allocation and heavy object churn inside the render loop.
- Treat loaders and post-processing passes as owned resources with explicit lifecycle.

## Architecture

- Model the scene graph around real game or product concepts, not just raw mesh names.
- Keep app state outside the Three.js object graph when broader application logic needs to reason about it.
- Use helpers and abstractions sparingly; the render pipeline should still be traceable by reading the code.
- Gate expensive effects behind measured value and user need.

## Verification

- Test math, interaction mapping, and asset pipeline logic outside the renderer where possible.
- Verify resize behavior, device pixel ratio handling, and reduced-performance devices.
- Profile frame time before tuning shaders, draw calls, or culling strategies.
- Provide fallbacks or graceful degradation for unsupported features and lower-end hardware.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- One giant file that creates the scene, handles input, loads assets, and runs gameplay logic.
- Leaking GPU resources by forgetting disposal or listener cleanup.
- Per-frame object creation when cached vectors or matrices would do.
- Visual effect stacking that kills clarity or frame rate for marginal payoff.

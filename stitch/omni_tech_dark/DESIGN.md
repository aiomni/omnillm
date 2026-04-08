# Design System: High-Performance Editorial for Rust Libraries

## 1. Overview & Creative North Star
**The Creative North Star: "The Architectural Compiler"**

This design system moves beyond the "standard developer dashboard" to create an editorial, high-performance environment. It treats code as art and documentation as a premium reading experience. By blending the brutalist efficiency of Rust with a sophisticated, layered dark mode, we evoke the feeling of a high-end IDE meets a luxury tech journal. 

We break the "template" look through **intentional asymmetry**—using wide gutters and staggered content blocks—and a **high-contrast typography scale** that pits massive, airy display type against dense, razor-sharp monospaced data. This isn't just a library; it’s an engine of precision.

---

## 2. Colors & Surface Philosophy

The palette is rooted in deep obsidian tones, punctuated by "Tech Blue" (`#0969DA`) and "Rust Tertiary" (`#ffb4a6`). 

### The "No-Line" Rule
**Explicit Instruction:** 1px solid borders are strictly prohibited for sectioning. 
Structure is defined through **Tonal Transitions**. To separate the sidebar from the main canvas, transition from `surface` to `surface-container-low`. To highlight a code block, nest it within a `surface-container-high` wrapper. The eye should perceive boundaries through light and depth, not lines.

### Surface Hierarchy & Nesting
Treat the UI as a series of stacked, machined plates.
- **Base Layer:** `surface` (#0f1419) - The infinite void.
- **In-Page Sections:** `surface-container-low` (#171c22) - Subtle differentiation for large content blocks.
- **Interactive Cards/Modules:** `surface-container-highest` (#30353b) - Elements that demand immediate attention.

### The "Glass & Gradient" Rule
To inject "soul" into the technical aesthetic, use **Radial Glassmorphism**. Floating panels (like command palettes or hover tooltips) should use `surface-variant` at 60% opacity with a `24px` backdrop-blur. 
**Signature Texture:** Main CTAs or Hero backgrounds should utilize a subtle linear gradient: `primary-container` (#0969da) to a deep `on-primary-fixed-variant` (#004493) at a 135-degree angle.

---

## 3. Typography

The system utilizes a dual-font strategy: **Space Grotesk** for structural authority and **Inter** for legible utility, with a heavy reliance on **system-monospace** for the "geeky" soul of the project.

- **Display (Space Grotesk):** Large, airy, and rhythmic. Used for hero headers and section starts. The wide tracking in `display-lg` creates a sense of "performance headroom."
- **Headline & Title (Space Grotesk):** High-contrast and bold. These act as the "scaffolding" of the page.
- **Body & Labels (Inter):** Neutral and invisible. Inter provides the workhorse legibility required for long-form technical documentation.
- **Code (UI-Monospace):** The primary brand voice. In this system, monospaced type is not just for code—it is used for `label-sm` and `label-md` to reinforce the "developer-first" identity.

---

## 4. Elevation & Depth

### The Layering Principle
Hierarchy is achieved by "stacking" tonal tiers. 
*Example:* A code snippet (`surface-container-highest`) sits atop a documentation card (`surface-container-low`), which sits on the main page (`surface`). This creates a natural "lift" without the visual noise of shadows.

### Ambient Shadows
When a component must float (e.g., a dropdown), use an **Ambient Light Shadow**:
- **Blur:** 40px - 60px.
- **Opacity:** 8%.
- **Color:** Use a tinted version of `primary` (#adc6ff) instead of black. This mimics the glow of a high-end monitor in a dark room.

### The "Ghost Border" Fallback
If accessibility requires a container boundary, use a **Ghost Border**: `outline-variant` at 15% opacity. It should feel like a suggestion of an edge, not a hard stop.

---

## 5. Components

### Buttons
- **Primary:** High-gloss gradient (`primary` to `primary-container`). Sharp `md` (0.375rem) corners. Text is `on-primary`, all-caps, monospaced.
- **Secondary:** Transparent background with a "Ghost Border." On hover, the background fills to `surface-container-highest`.
- **Tertiary:** Purely typographic. Monospaced `label-md` with an `on-tertiary` underline that expands on hover.

### Code Blocks & Cards
**Forbid the use of divider lines.**
Use vertical whitespace from the spacing scale (e.g., `48px` between logical sections). Code blocks use `surface-container-lowest` for the background to create a "sunken" terminal effect.

### Input Fields
Minimalist "Underline" style or "Ghost Box." Focus states should not change the border color to a thick line; instead, the background should shift from `surface-container` to `surface-bright`.

### Additional Signature Components
- **The "Rust Status" Chip:** A small, monospaced badge using `tertiary-container` and `on-tertiary-container` to indicate crate versions or build status.
- **Performance Meter:** A slim, 2px tall horizontal progress bar using a gradient from `primary` to `tertiary` to visualize data processing or library benchmarks.

---

## 6. Do's and Don'ts

### Do
- **Do** use `display-lg` typography for page titles, offset to the left with significant whitespace to the right.
- **Do** use monospaced fonts for all "meta-data" (timestamps, version numbers, tags).
- **Do** lean into asymmetry. A 60/40 split grid feels more "engineered" than a centered 50/50 layout.

### Don't
- **Don't** use pure black (#000000). Use the `surface` palette to maintain "tonal depth."
- **Don't** use standard icons (like Material or FontAwesome) in their default state. Opt for ultra-thin (1pt) stroke icons or monospaced character symbols (e.g., `->`, `::`, `[]`).
- **Don't** use standard "drop shadows." If it doesn't feel like light emitting from a screen, it's too heavy.
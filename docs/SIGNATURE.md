---
name: signature-design
version: 1.0.0
description: |
  Personal design system combining retro brutalism with CRT-era nostalgia.
  Use when generating UI code, styling components, or creating design tokens.
  Features burgundy/cream palette, heavy borders, monospace typography,
  scanline effects, and offset shadows.
globs:
  - "**/*.css"
  - "**/*.scss"
  - "**/*.tsx"
  - "**/*.jsx"
  - "**/tailwind.config.*"
allowed-tools:
  - Read
  - Write
  - Edit
---

# Signature Design System

You are a UI designer implementing a retro-brutalist aesthetic with CRT nostalgia. Apply this design system when generating styles, components, or design tokens.

---

## DESIGN PHILOSOPHY

| Element      | Approach                        |
| ------------ | ------------------------------- |
| Aesthetic    | Retro brutalism + CRT nostalgia |
| Feel         | Raw, functional, nostalgic      |
| Typography   | Monospace, terminal-inspired    |
| Interactions | Mechanical, tactile             |
| Personality  | Bold, unapologetic, distinctive |

---

## COLOR SYSTEM

### Light Mode

```css
:root {
  --color-primary: rgb(169, 56, 56); /* Burgundy */
  --color-secondary: rgb(236, 236, 222); /* Cream */
  --color-background: rgb(236, 236, 222);
  --color-text: rgb(169, 56, 56);
  --color-success: #16a085; /* Teal */
  --color-warning: #d97706; /* Amber */
  --color-neutral: #666;
  --color-accent: #3b82f6; /* Blue */
}
```

### Dark Mode

```css
[data-theme="dark"] {
  --color-primary: rgb(255, 120, 120); /* Coral */
  --color-secondary: rgb(30, 30, 35);
  --color-background: rgb(18, 18, 22); /* Charcoal */
  --color-text: rgb(255, 120, 120);
  --color-success: #2dd4bf;
  --color-warning: #fbbf24;
  --color-neutral: #9ca3af;
  --color-accent: #60a5fa;
}
```

### Color Usage

| Context     | Variable             |
| ----------- | -------------------- |
| Body text   | `--color-text`       |
| Headings    | `--color-primary`    |
| Backgrounds | `--color-background` |
| Borders     | `--color-primary`    |
| Interactive | `--color-success`    |
| Warnings    | `--color-warning`    |

---

## TYPOGRAPHY

### Font Stack

```css
font-family: "Courier New", monospace;
```

Monospace is mandatory. This is the core visual identity.

### Type Scale

```css
h1 {
  font-size: 2.5rem;
}
h2 {
  font-size: 2rem;
}
h3 {
  font-size: 1.5rem;
}
body {
  font-size: 1rem;
  line-height: 1.6;
}
```

### Typography Rules

```css
h1,
h2,
h3,
h4,
h5,
h6 {
  font-weight: bold;
  text-transform: uppercase;
  color: var(--color-primary);
  letter-spacing: 2px;
}
```

---

## SPACING

```css
:root {
  --spacing-xs: 8px;
  --spacing-sm: 12px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;
  --spacing-xxl: 48px;
}
```

---

## SHADOWS

### Standard

```css
:root {
  --shadow-card: 0 4px 7px rgba(40, 40, 40, 1);
  --shadow-hover: 0 6px 12px rgba(40, 40, 40, 1);
}
```

### Offset Shadow (Signature Style)

```css
.element:hover {
  transform: translateY(-2px);
  box-shadow: 4px 4px 0px var(--color-primary);
}
```

The offset shadow creates a "lifted" mechanical effect. This is a defining element.

---

## BORDERS

```css
:root {
  --border-radius: 10px;
  --border-width: 3px;
}

.card {
  border: var(--border-width) solid var(--color-primary);
  border-radius: var(--border-radius);
}
```

Borders are intentionally heavy and prominent.

---

## COMPONENTS

### Retro Button

```css
.btn {
  background: var(--color-background);
  border: var(--border-width) solid var(--color-primary);
  color: var(--color-primary);
  padding: 8px 16px;
  font-family: inherit;
  font-weight: bold;
  text-transform: uppercase;
  letter-spacing: 2px;
  cursor: pointer;
  transition:
    transform 0.2s ease,
    box-shadow 0.2s ease;
  box-shadow: var(--shadow-card);
  border-radius: 5px;
}

.btn:hover {
  transform: translateY(-2px);
  box-shadow: 4px 4px 0px var(--color-primary);
}

.btn:active {
  transform: scale(0.98);
}
```

### Card

```css
.card {
  background: var(--color-background);
  border: var(--border-width) solid var(--color-primary);
  border-radius: var(--border-radius);
  box-shadow: var(--shadow-card);
  padding: var(--spacing-lg);
}
```

### Header

```css
.header {
  height: 60px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 2rem;
  border-bottom: var(--border-width) solid var(--color-primary);
  box-shadow: var(--shadow-card);
  background: var(--color-background);
}
```

### Footer

```css
.footer {
  height: 30px;
  border-top: 1px solid var(--color-primary);
  padding: 0 1rem;
  font-size: 0.8rem;
  color: var(--color-secondary);
  background: var(--color-primary);
}
```

---

## CRT EFFECT

### Scanline Overlay

```css
.crt::before {
  content: "";
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background:
    linear-gradient(rgba(18, 16, 16, 0) 50%, rgba(0, 0, 0, 0.25) 50%),
    linear-gradient(
      90deg,
      rgba(255, 0, 0, 0.06),
      rgba(0, 255, 0, 0.02),
      rgba(0, 0, 255, 0.06)
    );
  background-size:
    100% 2px,
    3px 100%;
  pointer-events: none;
  z-index: 9999;
}
```

### Flicker Animation

```css
.crt::after {
  content: "";
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(18, 16, 16, 0.1);
  opacity: 0;
  pointer-events: none;
  z-index: 9999;
  animation: flicker 0.15s infinite;
}

@keyframes flicker {
  0% {
    opacity: 0.028;
  }
  10% {
    opacity: 0.042;
  }
  20% {
    opacity: 0.015;
  }
  30% {
    opacity: 0.008;
  }
  40% {
    opacity: 0.019;
  }
  50% {
    opacity: 0.032;
  }
  60% {
    opacity: 0.016;
  }
  70% {
    opacity: 0.038;
  }
  80% {
    opacity: 0.019;
  }
  90% {
    opacity: 0.032;
  }
  100% {
    opacity: 0.016;
  }
}
```

---

## GRID BACKGROUND

```css
.grid-bg {
  background-image:
    linear-gradient(rgba(169, 56, 56, 0.1) 1px, transparent 1px),
    linear-gradient(90deg, rgba(169, 56, 56, 0.1) 1px, transparent 1px);
  background-size: 50px 50px;
  background-position: center;
}
```

---

## ANIMATIONS

```css
/* Subtle lift */
transition:
  transform 0.2s ease,
  box-shadow 0.2s ease;

/* Fade in */
@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

/* Pulse for status indicators */
@keyframes pulse {
  0%,
  100% {
    opacity: 0.7;
  }
  50% {
    opacity: 1;
  }
}

/* Spin for loading */
@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
```

---

## ALTERNATIVE PALETTES

### Terminal Green

```css
--color-primary: rgb(0, 255, 65);
--color-background: rgb(10, 10, 10);
```

### Amber Terminal

```css
--color-primary: rgb(255, 176, 0);
--color-background: rgb(15, 10, 5);
```

### Corporate Blue

```css
--color-primary: rgb(59, 130, 246);
--color-background: rgb(15, 23, 42);
```

---

## COMPLETE CSS TEMPLATE

```css
:root {
  --color-primary: rgb(169, 56, 56);
  --color-secondary: rgb(236, 236, 222);
  --color-background: rgb(236, 236, 222);
  --color-text: rgb(169, 56, 56);
  --color-success: #16a085;
  --color-warning: #d97706;

  --spacing-xs: 8px;
  --spacing-sm: 12px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;

  --shadow-card: 0 4px 7px rgba(40, 40, 40, 1);
  --shadow-hover: 0 6px 12px rgba(40, 40, 40, 1);

  --border-radius: 10px;
  --border-width: 3px;
}

[data-theme="dark"] {
  --color-primary: rgb(255, 120, 120);
  --color-secondary: rgb(30, 30, 35);
  --color-background: rgb(18, 18, 22);
  --color-text: rgb(255, 120, 120);
  --color-success: #2dd4bf;
  --color-warning: #fbbf24;

  --shadow-card: 0 4px 7px rgba(0, 0, 0, 0.8);
  --shadow-hover: 0 6px 12px rgba(0, 0, 0, 0.9);
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: "Courier New", monospace;
  background-color: var(--color-background);
  color: var(--color-text);
  line-height: 1.6;
}

h1,
h2,
h3,
h4,
h5,
h6 {
  font-weight: bold;
  text-transform: uppercase;
  color: var(--color-primary);
  letter-spacing: 2px;
}

.btn {
  background: var(--color-background);
  border: var(--border-width) solid var(--color-primary);
  color: var(--color-primary);
  padding: 8px 16px;
  font-family: inherit;
  font-weight: bold;
  text-transform: uppercase;
  letter-spacing: 2px;
  cursor: pointer;
  transition:
    transform 0.2s ease,
    box-shadow 0.2s ease;
  box-shadow: var(--shadow-card);
  border-radius: 5px;
}

.btn:hover {
  transform: translateY(-2px);
  box-shadow: 4px 4px 0px var(--color-primary);
}
```

---

## TAILWIND CONFIG

```js
module.exports = {
  theme: {
    extend: {
      colors: {
        primary: "rgb(169, 56, 56)",
        secondary: "rgb(236, 236, 222)",
      },
      fontFamily: {
        mono: ['"Courier New"', "monospace"],
      },
      boxShadow: {
        retro: "4px 4px 0px rgb(169, 56, 56)",
        card: "0 4px 7px rgba(40, 40, 40, 1)",
      },
    },
  },
};
```

---

## SUMMARY

Apply these elements for the signature style:

1. **Burgundy/Cream palette** with dark mode variant
2. **Heavy borders** (3px solid)
3. **Monospace typography** (Courier New)
4. **Offset shadows** (4px 4px 0px)
5. **CRT effects** (scanlines, flicker)
6. **Grid overlays**
7. **Uppercase headings** with letter-spacing

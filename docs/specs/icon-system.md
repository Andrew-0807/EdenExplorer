# Spec Card: Axis Icon System

## 1. Overview
Defines the visual language, geometry, grid rules, and production deliverables for the **Axis** file manager iconography based on `.agents/skills/icon-system`.

## 2. Grid & Geometry Standards
- **Base Grid**: 24x24px, 2px internal padding (20x20px active live area).
- **Stroke**: Uniform 2.0px stroke width, rounded caps (`stroke-linecap="round"`), rounded joins (`stroke-linejoin="round"`).
- **Corner Radii**: 2.0px standard curvature for sharp geometric rhythm.
- **Keyline Silhouettes**:
  - Circle: 20x20px centered at (12, 12).
  - Square: 18x18px centered at (12, 12).
  - Vertical Rectangle: 20x16px.
  - Horizontal Rectangle: 16x20px.

## 3. Core Motif: The Coordinate Axis Monogram
- **Visual Thesis**: Unifies the letter **A** with a 3-point Cartesian coordinate origin.
- **Components**:
  - Vector Legs: Diagonal coordinate vectors radiating from apex (12, 3) to base terminals (4, 20) and (20, 20).
  - Abscissa (X-Axis): Horizontal transverse datum at Y=14.
  - Ordinate (Y-Axis): Vertical normal vector descending from apex (12, 3) to origin (12, 14).
  - Origin Marker: Focal coordinate node at (12, 14).

## 4. Size & Target Scaling Matrix
| Size Tier | Dimensions | Stroke | Use Case |
|---|---|---|---|
| XS | 16x16px | 1.5px | Tree-view item, dense file table status |
| S  | 20x20px | 1.75px| Compact toolbar buttons, tab headers |
| M  | 24x24px | 2.0px | Standard UI navigation & command bar |
| L  | 32x32px | 2.5px | Dialog headers, sidebar primary toggles |
| XL | 48x48px+ | Variable | Windows shell tile, installer banner |
| App Tile | 256x256px | Scaled | OS Application icon (`.ico`, Start Menu, Taskbar) |

## 5. File Deliverables & Naming
- `src/assets/icons/axis/icon-app-axis-glyph.svg`: 24x24 clean vector glyph (Lucide compatible).
- `src/assets/icons/axis/icon-app-axis-tile.svg`: 256x256 desktop squircle application tile.

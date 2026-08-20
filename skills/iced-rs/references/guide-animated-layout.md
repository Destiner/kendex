# Animated Layout Transitions

Patterns for animating between collapsed and expanded states in custom widgets. Covers measured positions, keyed identity, geometry-driven travel, and transition clipping.

**Read order**: `animation.md` → this guide → `guide-custom-widgets.md` → `advanced-tree.md` → `advanced-layout.md`.

**When to read**: before building any UI with animated expand/collapse, reorderable layered views, floating panels, toasts, or variable-height animated lists.

## Measured positions over estimated heights

Synthetic per-item height estimates (e.g. "each item is ~48px") are a fragile source of truth for final animated layout spacing.

**Acceptable**: height hints for rough travel budgets, fallback heuristics, or initial layout before measurement.

**Not acceptable**: authoritative source for expanded layout spacing when child heights vary.

Mixed content — items with icons, items without, description rows, button rows, close buttons, custom embedded content — will drift from any single estimate. Repeated one-off fixes for specific item variants is a signal the architecture needs measured child layout, not more estimates.

### What to do instead

1. **Measure child layout.** Use `layout()` to obtain the real `layout::Node` for each child. The node's `bounds().height` is the authoritative expanded height.
2. **Use a real list-layout source of truth.** Store measured expanded geometry alongside collapsed geometry in Tree state. Interpolate between the two.
3. **Interpolate from collapsed geometry to measured expanded geometry** — not from collapsed to an estimate.

```rust
// In Tree state
struct ItemGeometry {
    collapsed_y: f32,
    expanded_y: f32,
    expanded_height: f32,
}

// During layout, measure each child's real expanded size
fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
    let state = tree.state.downcast_mut::<ListState>();
    let mut y = 0.0;

    for (i, child) in self.children.iter_mut().enumerate() {
        let child_node = child.layout(&mut tree.children[i], renderer, &child_limits);
        let measured_height = child_node.bounds().height;

        state.items[i].expanded_y = y;
        state.items[i].expanded_height = measured_height;
        y += measured_height + self.spacing;
    }
    // ... assemble final Node with children
}
```

> **Warning**: if you find yourself adding special-case height adjustments for specific content variants ("add 24px when the item has an icon," "add 32px for the close button row"), the architecture needs measured child layout, not more estimates.

## Collapsed-to-expanded transition pattern

Animating between two real layouts (e.g. a stacked collapsed view and a fully expanded list) requires a disciplined pattern.

### The pattern

1. **One stable source of item identity.** Each item has a unique key that survives reordering, insertion, and removal.
2. **One stable hover sensor.** The hitbox that triggers expand/collapse does not change size during the animation. See `guide-custom-widgets.md` § "Stable hover hit regions."
3. **Measured expanded positions.** Run child `layout()` to get real sizes, store in Tree state.
4. **Collapsed positions derived from stack geometry.** Compute where each item sits in the collapsed presentation from the stack's geometry rules (overlap offset, stacking direction).
5. **Interpolate between the two.** Use a single `Animation<f32>` or `Animation<bool>` (0.0 = collapsed, 1.0 = expanded) to lerp positions and sizes.

```rust
// Tree state for the transition
#[derive(Default)]
struct CollapseState {
    expanded: Animation<bool>,
    items: Vec<ItemGeometry>,
}

struct ItemGeometry {
    collapsed_y: f32,
    expanded_y: f32,
    measured_height: f32,
}

impl ItemGeometry {
    fn y_at(&self, t: f32) -> f32 {
        self.collapsed_y + (self.expanded_y - self.collapsed_y) * t
    }
}

// In draw(), interpolate positions using the transition parameter
fn draw(&self, tree: &Tree, renderer: &mut Renderer, /* ... */) {
    let state = tree.state.downcast_ref::<CollapseState>();
    let now = /* current Instant */;
    let t = state.expanded.interpolate_with(
        |b| if *b { 1.0 } else { 0.0 }, now,
    );

    for (i, (child, child_tree)) in self.children.iter()
        .zip(tree.children.iter())
        .enumerate()
    {
        let y = state.items[i].y_at(t);
        renderer.with_translation(Vector::new(0.0, y), |renderer| {
            child.draw(child_tree, renderer, /* ... */);
        });
    }
}
```

### What to avoid

**Duplicated animated trees in a `stack`.** Do not overlay a full collapsed tree and a full expanded tree in a `stack` and crossfade between them.

Problems:
- Both trees process events — hover, click, and drag fire on both layers simultaneously
- Identity is duplicated — each item exists twice in the widget tree, causing state confusion during `diff()`
- Opacity crossfade creates visible overlap artifacts during the transition
- Layout invalidation affects both trees, compounding performance cost

```rust
// BAD: two full trees crossfading
stack![
    container(collapsed_list).opacity(1.0 - t),  // full tree with event handlers
    container(expanded_list).opacity(t),           // duplicate tree, duplicate events
]

// GOOD: one tree, interpolated positions
my_animated_list(items)
    .expanded(self.is_expanded)  // single source of truth
```

**Branch-swap with no real transition.** Do not use `if expanded { expanded_view() } else { collapsed_view() }` without intermediate states.

Problems:
- No animation — the view pops between states
- Widget tree shape changes, resetting all persistent state (scroll positions, hover, animation progress)
- Violates the widget tree consistency rule (see `../SKILL.md` § "Widget tree consistency")

```rust
// BAD: branch swap — pops, loses state
if self.expanded {
    expanded_list(items).into()
} else {
    collapsed_stack(items).into()
}

// GOOD: single widget, animated t parameter
animated_list(items).expanded(self.expanded)
```

**Fixed expanded-height hitbox as hover source.** Do not use a `mouse_area` the size of the expanded content to detect hover for collapse/expand.

Problems:
- When collapsed, the hitbox extends beyond visible content — adjacent UI competes for hover
- Users trigger expansion by mousing over empty space below the collapsed view

## Keyed identity in reordered and layered views

When items can be reordered, removed, or presented in different visual modes (list view, grid, stacked layers), their identity must remain stable across all presentations.

### The problem

If you build a collapsed layered view by sorting children by Z-position and iterating `self.children[sorted_indices[i]]`, but the Tree children are stored in insertion order, Tree child `i` no longer corresponds to visual child `i`. Symptoms:
- Wrong icons or content appear on items after reorder
- Stale subtree state (old hover, old animation phase) appears on the wrong item
- Removing an item causes a different item to reset or display stale content

### The rule

**Keyed identity must remain stable in all presentation modes.** The association between an item's unique key and its Tree child index must not change based on visual ordering.

Options:
1. **Use `keyed::Column`** for list presentations — it diffs by key, not position. See `widget-lazy-keyed.md`.
2. **Maintain a stable key→index mapping** in Tree state. When reordering visually, reorder draw calls (not child indices).
3. **Use `Tree::diff_children_custom`** with key-aware reconciliation when items are added, removed, or reordered. See `advanced-tree.md`.

```rust
// GOOD: stable Tree child order, reordered draw calls
fn draw(&self, tree: &Tree, renderer: &mut Renderer, /* ... */) {
    let state = tree.state.downcast_ref::<MyState>();

    // Visual order may differ from Tree child order
    for &draw_index in &state.visual_order {
        let child = &self.children[draw_index];
        let child_tree = &tree.children[draw_index];
        child.draw(child_tree, renderer, /* ... */);
    }
}
```

### Testing guidance

- **Reorder with distinct content.** Give each item visibly different icons, text, and colors. After reorder, verify each item displays its own content — not the content of the item previously at that position.
- **Remove and verify neighbors.** Remove item N, confirm item N+1 doesn't inherit N's stale state (hover highlight, animation phase, expanded/collapsed state).
- **Switch presentation modes.** Toggle between list and layered view. Confirm each item retains its identity, animation state, and correct content.
- **Inspect Tree children count.** After removal, `tree.children.len()` must match the current item count. Orphaned Tree children cause stale rendering.

## Geometry-driven entry and exit travel

Entry/exit travel distance for floating or sliding UI (toasts, banners, popovers, floating panels) must be derived from measured geometry, not a fixed pixel constant.

### Why

A fixed travel distance (e.g. `const SLIDE_DISTANCE: f32 = 60.0`) works only when all items are the same height. Variable-height content will:
- **Clip** if the travel is shorter than the item
- **Leave visible remnants** if the travel doesn't fully clear the visible area
- **Look inconsistent** across different-sized items in the same list

### The rule

Derive travel from the item's measured size or a conservative geometry contract:

```rust
// GOOD: travel derived from measured height
let travel = item.measured_height + margin;
let offset_y = travel * (1.0 - t);  // t: 0→1 during entry

// BAD: fixed travel regardless of content
const SLIDE_DISTANCE: f32 = 60.0;
let offset_y = SLIDE_DISTANCE * (1.0 - t);  // clips tall items, wastes space on short ones
```

For items whose size isn't known until layout, use the measured `layout::Node` bounds stored in Tree state as the travel source.

### Applies to

Toasts, notification banners, popovers, sliding panels, bottom sheets, dropdown menus, snackbars.

## Transition height and clipping

Animating between collapsed and expanded states requires both interpolated positions **and** correct current visible bounds / clipping.

### The problem

Position interpolation alone is insufficient:
- If a widget always reports its expanded height to the parent layout, trailing content remains visible even when the widget is visually collapsed
- If collapsed content should show only stacked slivers, but the widget's reported bounds include the full expanded height, children outside the collapsed footprint are not clipped

### The rule

During a transition, the widget's reported layout size and any clipping region must match its current animated state:

```rust
fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
    let state = tree.state.downcast_mut::<CollapseState>();
    let now = Instant::now();
    let t = state.expanded.interpolate_with(
        |b| if *b { 1.0 } else { 0.0 }, now,
    );

    let collapsed_height = self.collapsed_height();
    let expanded_height = self.measure_expanded_height(tree, renderer, limits);
    let current_height = collapsed_height + (expanded_height - collapsed_height) * t;

    // Report current animated height to parent — not always-expanded
    Node::new(Size::new(limits.max().width, current_height))
}
```

In `draw()`, clip children to the current animated bounds:

```rust
fn draw(&self, tree: &Tree, renderer: &mut Renderer, /* ... */) {
    let state = tree.state.downcast_ref::<CollapseState>();
    let current_height = /* interpolated height from state */;

    let clip = Rectangle {
        x: layout.bounds().x,
        y: layout.bounds().y,
        width: layout.bounds().width,
        height: current_height,
    };

    renderer.with_layer(clip, |renderer| {
        // Draw children — only visible within clip bounds
        for (child, child_tree) in self.children.iter().zip(tree.children.iter()) {
            child.draw(child_tree, renderer, /* ... */);
        }
    });
}
```

### Diagnostic

If content is visible below where it should be during collapse:
1. Is `layout()` returning the expanded height unconditionally?
2. Is `draw()` clipping to the current animated bounds with `renderer.with_layer()`?
3. Is `shell.invalidate_layout()` being called each animation frame so `layout()` re-runs?

## Anti-patterns summary

| Anti-pattern | Problem | Fix |
|---|---|---|
| Duplicated animated trees in `stack` | Double events, identity confusion, overlap artifacts | Single tree, interpolated positions |
| Branch-swap (`if expanded { A } else { B }`) | No transition, tree shape change resets state | Single tree with animated `t` parameter |
| Estimated heights as final spacing | Drift with mixed content, repeated one-off fixes | Measured child layout |
| Hover sensor on animated bounds | Enter/exit thrash during animation | Stable outer hitbox (see `guide-custom-widgets.md`) |
| Fixed travel distance | Clip or remnant with variable-height items | Geometry-derived travel |
| Always-expanded layout height | Trailing content visible when collapsed | Animated layout height + `with_layer` clipping |
| Position-based child reordering | Stale subtrees, wrong content after reorder | Key-stable identity mapping |

## See also

- `animation.md` — animation rules, `Animation<T>`, redraw scheduling, redraw vs rebuild invariant
- `guide-custom-widgets.md` — Widget impl checklist, draw order, hover stability, opacity semantics
- `guide-animation-debugging.md` — debugging checklist for animation/render bugs
- `advanced-tree.md` — Tree state, Tag, `diff_children_custom`
- `advanced-layout.md` — `layout::Node`, `Limits`
- `widget-lazy-keyed.md` — `keyed::Column` for key-based diffing
- `examples/toast/src/main.rs` — animated overlay with lifecycle, entry/exit
- `examples/loading_spinners/src/circular.rs` — custom Widget with Tree-state animation loop

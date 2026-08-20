# Animation and Rendering Debugging Checklist

Quick-reference checklist for diagnosing animation and rendering bugs in custom Iced widgets.

**When to read**: any time an animation or rendering issue doesn't resolve after checking the obvious (`invalidate_layout`, `request_redraw`, event capture).

## Master checklist

### 1. Is motion state in Tree state or stale widget fields?

`request_redraw()` repaints the existing widget tree — it does **not** call `view()` to rebuild widgets. If animation state is stored in widget struct fields computed in `view()`, redraws repaint stale values.

**Check**: is the animated value in `tree.state.downcast_mut::<State>()`? If it's a field on the widget struct, it won't update between `view()` calls.

### 2. Is redraw happening without rebuild?

If animation depends on recomputing values in `view()` or app-level `update()`, a `request_redraw()` loop won't help — you need messages or tasks that trigger `update()` → `view()`.

**Check**: does the animation advance through `RedrawRequested` events handled in `Widget::update()`, or does it depend on `App::update()` recomputing state?

### 3. Is draw order correct?

In custom widget `draw()`, child iteration order is the z-order. The last child drawn appears on top. `stack` semantics do not apply automatically inside manual draw loops.

**Check**: are layered children drawn in the intended order? Background first, foreground last?

### 4. Is hover attached to animated bounds?

If `mouse_area` or a hover sensor wraps content whose size changes during a hover-triggered animation, the sensor boundary changes during animation, causing enter/exit event thrashing and flicker.

**Check**: is the hover hitbox stable, or does it grow/shrink during the animation it triggers?

### 5. Are there duplicated animated branches?

Overlaying two full animated widget trees (collapsed + expanded) in a `stack` and crossfading creates double event handling, identity duplication, and overlap artifacts.

**Check**: does the widget tree contain two copies of the same logical content?

### 6. Is final spacing measured or guessed?

If expanded item heights are estimated rather than measured from child layout, mixed content (icons, buttons, descriptions, variable text) will have incorrect spacing.

**Check**: does the layout use `child_node.bounds().height` from actual `layout()` calls, or a constant/estimate?

### 7. Is current visible height/clipping correct during transitions?

If a widget always reports expanded height even while animating toward collapsed, content below the collapsed footprint remains visible.

**Check**: does `layout()` return the current animated height? Does `draw()` clip with `renderer.with_layer()`?

### 8. Are all child rendering paths participating in opacity?

If a component fades as one unit, every child must fade: text, backgrounds, buttons, icons, canvas content, SVGs, images, custom shader output.

**Check**: set opacity to near-zero — are there any visible remnants? Those come from rendering paths outside the fade contract.

### 9. Are keyed identities stable across reorder/removal?

Reordering visual presentation by sorting child indices without updating Tree associations causes stale subtrees to appear on the wrong items.

**Check**: after reordering, does each item still show its own content (icon, text, state)?

### 10. Are SVG/canvas/image paths obeying the same fade semantics?

SVG tint alpha, canvas program colors, and image opacity may not behave identically to text/container color alpha in all renderers.

**Check**: when fading SVG icons or canvas content, does the actual rendered alpha match the expected value? Test at near-zero opacity.

### 11. Is entry/exit travel derived from actual geometry?

Fixed-pixel travel distances cause clipping or visible remnants with variable-height content.

**Check**: is slide/travel distance derived from the item's measured height, or a constant?

## Symptom quick lookup

| Symptom | Checks | Likely cause |
|---|---|---|
| Animation doesn't advance after first frame | 1, 2 | State in widget fields, not Tree; or redraw without rebuild |
| Layering looks inverted (background on top) | 3 | Wrong draw order in `draw()` |
| Hover flickers / thrashes during animation | 4 | Hover sensor on animated bounds |
| Double-click or double-event on animated items | 5 | Duplicated animated trees in `stack` |
| Spacing wrong for some items but not others | 6 | Estimated heights, not measured |
| Content visible below collapsed widget | 7 | Layout reports expanded height; no clipping |
| Near-transparent remnants after fade | 8, 10 | Partial fade contract; SVG/canvas not fading |
| Wrong content on items after reorder | 9 | Position-based identity, not key-based |
| Items clip during entry/exit animation | 11 | Fixed travel distance |
| "Second click" / delayed visual response | 1, 7 | Stale layout or stale widget fields |
| Widget "only updates on second click" | 7 | Missing `invalidate_layout()` on state change |

## See also

- `animation.md` — animation rules, redraw scheduling, redraw vs rebuild
- `guide-custom-widgets.md` — Widget impl checklist, draw order, hover stability, opacity
- `guide-animated-layout.md` — measured positions, collapsed/expanded transitions, keyed identity
- `advanced-tree.md` — Tree state persistence
- `advanced-shell.md` — Shell: `request_redraw`, `invalidate_layout`, `capture_event`

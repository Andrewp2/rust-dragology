//! Simulate dragging the banana row by its label. All coordinates are in list space.
use dragology::{Affine, DragError, DragSession, Point, Preview, closest};

fn main() -> Result<(), DragError> {
    let items = vec!["apple", "banana", "cherry", "date"];
    let index = 1;
    let dragged = items[index];
    let layout = |order: &Vec<&str>| {
        let row = order.iter().position(|&item| item == dragged)?;
        Some(Affine::translate((0.0, row as f64 * 50.0)))
    };

    let orders = (0..items.len()).map(|destination| {
        let mut order = items.clone();
        order.remove(index);
        order.insert(destination, dragged);
        order
    });
    let spec = closest(orders).with_floating();
    let mut drag = DragSession::begin(items, (12.0, 56.0), layout, spec)?;

    for y in [56.0, 92.0, 149.0] {
        let frame = drag.frame((12.0, y))?;
        if let Preview::Floating {
            backdrop,
            source,
            translation,
            ..
        } = frame.preview.as_ref()
            && let Preview::State(background) = backdrop.as_ref()
            && let Preview::State(foreground) = source.as_ref()
        {
            // A UI draws the other rows in their backdrop slots, then draws
            // the banana separately at its translated origin.
            let origin = layout(foreground).unwrap() * Point::ORIGIN + *translation;
            println!(
                "Pointer y={y:3.0}   slots {background:?}   {dragged} y={:.0}",
                origin.y
            );
        }
    }
    let order = drag.finish((12.0, 149.0))?;
    println!("Dropped order   {order:?}");
    Ok(())
}

//! Smoke test: Verify the actor model with non-blocking input.
//!
//! This example will be expanded in Phase 3.

fn main() {
    println!("Flywheel Smoke Test");
    println!("===================");
    println!();
    println!("Cell size: {} bytes", std::mem::size_of::<flywheel::Cell>());
    println!("Rgb size:  {} bytes", std::mem::size_of::<flywheel::Rgb>());
    println!();

    // Create a buffer
    let buffer = flywheel::Buffer::new(80, 24);
    println!("Buffer: {}x{} = {} cells", buffer.width(), buffer.height(), buffer.len());
    println!("Memory: {} bytes", buffer.memory_usage());
    println!();

    // Test cell creation
    let cell = flywheel::Cell::new('X')
        .with_fg(flywheel::Rgb::new(255, 100, 50))
        .with_bg(flywheel::Rgb::new(20, 20, 30));
    println!("Created cell: {:?}", cell);
    println!();

    println!("Phase 1 complete: Core primitives working!");
}

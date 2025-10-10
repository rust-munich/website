use tailwind_css::TailwindBuilder;

fn main() {
    let mut tailwind = TailwindBuilder::default();
    // The compiler will expand directly into the final css property
    // Inline style will not be tracked
    let inline = tailwind.inline("py-2 px-4 bg-green-500");
    // The compiler will expand into a `class`, and record the style class used
    tailwind.trace("py-2 px-4 bg-green-500", false);
    // Compile all traced classes into bundle
    let bundle = tailwind.bundle();
}

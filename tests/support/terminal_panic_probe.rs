fn main() {
    ratatui::try_init().expect("could not initialize terminal");
    panic!("intentional terminal restoration probe");
}

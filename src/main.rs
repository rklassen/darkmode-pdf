mod constants;
mod model;
mod view;
mod viewmodel;

fn main() -> anyhow::Result<()> {
    viewmodel::app::run()
}

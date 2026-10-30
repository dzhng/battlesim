use super::*;

pub(super) struct App {
    pub(super) args: Args,
    pub(super) run: Option<Run>,
    pub(super) error: Option<String>,
}

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.run.is_some() {
            return;
        }
        match Run::start(event_loop, &self.args) {
            Ok(run) => self.run = Some(run),
            Err(e) => self.fail(event_loop, e),
        }
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Occluded(occluded) => println!("window occluded: {occluded}"),
            _ => {}
        }
    }

    /// The frames run from the loop itself, not from redraw requests, which
    /// macOS doesn't deliver to a window it isn't showing.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(run) = self.run.as_mut() else { return };
        match run.step(&self.args) {
            Ok(true) => {}
            Ok(false) => {
                if let Err(e) = run.report(&self.args) {
                    self.error = Some(e);
                }
                self.run = None;
                event_loop.exit();
            }
            Err(e) => self.fail(event_loop, e),
        }
    }
}

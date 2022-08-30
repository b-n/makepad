use crate::{
    makepad_draw_2d::*,
    build::build_manager::BuildState,
    makepad_platform::os::cx_stdin::{
        HostToStdin,
        StdinToHost,
        StdinWindowSize
    },
    build::{
        build_protocol::*,
    }
};

live_register!{
    import makepad_draw_2d::shader::std::*;
    
    DrawApp: {{DrawApp}} {
        texture tex: texture2d
        fn pixel(self) -> vec4 {
            //return vec4(self.max_iter / 1000.0,0.0,0.0,1.0);
            let fb = sample2d_rt(self.tex, self.pos)
            if fb.r == 1.0 && fb.g == 0.0 && fb.b == 1.0 {
                return #4
            }
            return fb;
        }
    }
    RunView: {{RunView}} {
        frame_delta: 0.016
    }
}

#[derive(Live, LiveHook)]
#[repr(C)]
pub struct DrawApp {
    draw_super: DrawQuad,
}

#[derive(Live)]
pub struct RunView {
    bg: DrawApp,
    state: State,
    frame_delta: f64,
    #[rust] last_size: (usize, usize),
    #[rust] tick: Timer,
    #[rust] time: f64,
    #[rust] frame: u64
}

impl LiveHook for RunView {
    fn after_new_from_doc(&mut self, cx: &mut Cx) {
        self.tick = cx.start_interval(self.frame_delta);
        self.time = 0.0;
    }
}

impl RunView {
    
    pub fn handle_event(&mut self, cx: &mut Cx, event: &Event, state: &mut BuildState) {
        self.state_handle_event(cx, event);
        if self.tick.is_event(event) {
            self.time += self.frame_delta;
            self.frame += 1;
            
            // what shall we do, a timer? or do we do a next-frame
            state.send_host_to_stdin(None, HostToStdin::Tick {
                frame: self.frame,
                time: self.time
            })
        }
        // ok what do we want. lets do fingerdown, finger 
        
    }
    
    pub fn handle_stdin_to_host(&mut self, cx: &mut Cx, _cmd_id: BuildCmdId, msg: StdinToHost, _state: &mut BuildState) {
        match msg {
            StdinToHost::ReadyToStart => {
                // cause a resize event to fire
                self.last_size = Default::default();
                self.redraw(cx);
            }
            StdinToHost::DrawComplete => {
                self.bg.redraw(cx);
            }
        }
    }
    
    pub fn redraw(&mut self, cx: &mut Cx) {
        self.bg.area().redraw(cx);
    }
    
    pub fn draw(&mut self, cx: &mut Cx2d, state: &BuildState) {
        
        // alright so here we draw em texturezs
        // pick a texture off the buildstate
        let dpi_factor = cx.current_dpi_factor();
        let rect = cx.walk_turtle(Walk::fill()).dpi_snap(dpi_factor);
        // lets pixelsnap rect in position and size
        self.bg.draw_abs(cx, rect);
        for client in &state.clients {
            for process in client.processes.values() {
                let new_size = ((rect.size.x * dpi_factor) as usize, (rect.size.y * dpi_factor) as usize);
                if new_size != self.last_size {
                    self.last_size = new_size;

                    process.texture.set_desc(cx, TextureDesc {
                        format: TextureFormat::SharedBGRA(0),
                        width: Some(new_size.0),
                        height: Some(new_size.1),
                        multisample: None
                    });
                    
                    state.send_host_to_stdin(Some(process.cmd_id), HostToStdin::WindowSize(StdinWindowSize {
                        width: rect.size.x,
                        height: rect.size.y,
                        dpi_factor: dpi_factor,
                    }));
                }
                self.bg.set_texture(0, &process.texture);
                break
            }
        }
    }
}
use el320x240_36hb_sender::run;

fn main() {
    pollster::block_on(run());
}
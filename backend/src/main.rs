//! Lutra's backend: everything that touches the system (sources, launching, controllers,
//! artwork, the database), served to the Electron shell over stdin / stdout.

fn main() {
    lutra_lib::run()
}

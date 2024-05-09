use std::{env, io, os::unix::fs, process::ExitCode};
use unshare::{Command, Namespace};

fn main() -> io::Result<ExitCode> {
    println!("Hello, world!");
    // let args = env::args().collect::<Vec<String>>();
    let args = vec![
        String::from("docker"),
        // String::from("/nix/store/zk49qzjd42l93bx6dybavqj8vjkd0pyx-net-tools-2.10/bin/hostname"),
        // String::from("/nix/store/67z6w2mjmz2dpg7p84891bfwhip91gh4-busybox-1.36.1/bin/busybox"),
        // String::from("/store/67z6w2mjmz2dpg7p84891bfwhip91gh4-busybox-1.36.1/bin/busybox"),
        // String::from("/bin/busybox"),
        // String::from("busybox"),
        // String::from("ls"),
        String::from("/hello"),
        // String::from("."),
        // String::from("/"),
    ];
    let image = &args[1];
    let extra_args = Vec::from(&args[2..]);
    let mut cmd = Command::new(image);
    cmd.args(extra_args.as_ref());
    cmd.unshare(vec![
        &Namespace::Uts,
        &Namespace::Mount,
        &Namespace::Pid,
        &Namespace::User,
    ]);
    // cmd.current_dir("/tmp/nvim.black/");
    unsafe {
        cmd.pre_exec(|| {
            hostname::set("docker-container")?;
            fs::chroot("/tmp/wl")?;
            env::set_current_dir("/")?;
            Ok(())
        })
    };

    return cmd
        .status()
        .map(|status| ExitCode::from(status.code().unwrap_or(255) as u8))
        .map_err(|e| {
            println!("unshare error: {}", e);
            io::Error::from_raw_os_error(255)
        });
}

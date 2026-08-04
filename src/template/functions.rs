use crate::fakegen;
use crate::seed::RngHandle;
use minijinja::Environment;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// Scratch slot holding the RNG for the render currently in progress on
    /// this thread. Set immediately before `Template::render`, cleared
    /// immediately after — never left set across an await point or across
    /// requests. This is the standard workaround for minijinja global
    /// functions needing per-call state: functions are registered once,
    /// globally, but must read whichever RNG is "current" at call time.
    static CURRENT_RNG: RefCell<Option<Rc<RngHandle>>> = const { RefCell::new(None) };
}

/// Runs `f` with `rng` installed as the current thread's RNG, guaranteed to
/// clear the slot afterward even if `f` panics.
pub fn with_rng<T>(rng: Rc<RngHandle>, f: impl FnOnce() -> T) -> T {
    CURRENT_RNG.with(|c| *c.borrow_mut() = Some(rng));
    struct ClearOnDrop;
    impl Drop for ClearOnDrop {
        fn drop(&mut self) {
            CURRENT_RNG.with(|c| *c.borrow_mut() = None);
        }
    }
    let _guard = ClearOnDrop;
    f()
}

fn current_rng() -> Rc<RngHandle> {
    CURRENT_RNG.with(|c| {
        c.borrow()
            .clone()
            .expect("template global function called outside with_rng scope")
    })
}

pub fn register(env: &mut Environment<'static>) {
    env.add_function(
        "aws_access_key",
        || fakegen::aws::access_key(&current_rng()),
    );
    env.add_function("password", |len: u32| {
        fakegen::secrets::password(&current_rng(), len as usize)
    });
    env.add_function("hex", |len: u32| {
        fakegen::secrets::hex(&current_rng(), len as usize)
    });
    env.add_function("base64", |len: u32| {
        fakegen::secrets::base64_str(&current_rng(), len as usize)
    });
    env.add_function("jwt", || fakegen::secrets::jwt(&current_rng()));
    env.add_function(
        "private_ip",
        || fakegen::network::private_ip(&current_rng()),
    );
    env.add_function("uuid", || fakegen::identity::uuid(&current_rng()));
    env.add_function("hostname", || fakegen::network::hostname(&current_rng()));
    env.add_function("email", || fakegen::identity::email(&current_rng()));
}

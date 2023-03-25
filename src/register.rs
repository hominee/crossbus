//! Actor lifecycle Management
//!
extern crate alloc;

use crate::actor::Actor;
#[cfg(not(feature = "log"))]
use crate::log;
use alloc::{string::String, sync::Arc, vec::Vec};
use core::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    fmt, hint,
    mem::transmute,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// number of all actors that counts them
/// when a new actor is created, increase it
/// and use the current number as the id of the actor
static ACTORCOUNT: AtomicUsize = AtomicUsize::new(1);

/// Actor system register for all created actor
/// it serves as a index for all living actor of the system can
/// - query/update actor info(name/inner data/etc.)
/// - push new actor to
/// - delete stopped actor from
///
/// it built upon Vec which you use for iteration/filter/etc.
///
static mut REGISTER: Register = Register { inner: Vec::new() };
/// A atomic guard to protect mutate REGISTER
/// by multiple threads concurrently
static REGISTERSEAL: AtomicBool = AtomicBool::new(false);

/// An general record of running actors
///
/// when an actor get into running,
/// A record will registered into `Register` for future
/// usage (Create/Read/Update/Delete)
///
/// and the registered `ActorRegister` will be removed
/// when the actor is closed
pub struct Register {
    inner: Vec<ActorRegister>,
}

impl Register {
    /// create an instance of Register
    pub fn new() -> &'static Self {
        unsafe { &REGISTER }
    }

    /// push an actor into `REGISTER` and
    /// return its underlying guarded actor
    pub fn push<A: Actor>(item: ActorRegister) -> ActorGuard<A> {
        // the underlying actor is required to be downcasted;
        let data = item.downcast_ref_cell();
        while REGISTERSEAL.load(Ordering::Acquire) {
            hint::spin_loop();
        }
        REGISTERSEAL.store(true, Ordering::Release);
        log::debug!("actor registered: {:?}", item);
        unsafe {
            REGISTER.inner.push(item);
        }
        REGISTERSEAL.store(false, Ordering::Relaxed);
        data
    }

    /// get an actor register by id
    ///
    /// use [iter](Self::iter) instead if
    /// `message_id` or actor `type_id`
    /// is involved
    pub fn get(id: usize) -> Option<&'static ActorRegister> {
        Self::as_ref().iter().find(|reg| reg.id() == id)
    }

    /// iterate actor(s) register
    pub fn iter() -> core::slice::Iter<'static, ActorRegister> {
        Self::as_ref().iter()
    }

    /// clear closed actor as long as
    /// - `ActorRegister.closed` marked `true`
    /// - All other `ActorGuard` copys dropped
    pub(crate) fn update() -> Vec<ActorRegister> {
        let mut items = Vec::new();
        let mut index = 0;
        // wait for `REGISTER` to be available
        while REGISTERSEAL.load(Ordering::Acquire) {
            hint::spin_loop();
        }
        // seal it to get mutable reference
        REGISTERSEAL.store(true, Ordering::Release);
        let inner = unsafe { &mut REGISTER.inner };
        while index < inner.len() {
            let item = &mut inner[index];
            if item.is_closed() && Arc::strong_count(&item.inner) == 1 {
                let item = inner.swap_remove(index);
                log::debug!("Actor Register removed: {:?}", item);
                items.push(item);
                continue;
            }
            index += 1;
        }
        REGISTERSEAL.store(false, Ordering::Release);
        items
    }

    /// get the number of all created actors
    pub fn len() -> usize {
        unsafe { &REGISTER.inner }.len()
    }

    /// take the reference of Register
    pub fn as_ref() -> &'static Vec<ActorRegister> {
        //while REGISTERSEAL.load(Ordering::Relaxed) {
        //hint::spin_loop();
        //}
        unsafe { &REGISTER.inner }
    }
}

/// A record will registered into `REGISTER` for
/// future use (Create/Read/Update/Delete)
///
/// each record contains
/// - `id`: `usize`, the unique identifier of the actor.
/// - `actor`: the type implemented [`Acotr`](Actor) trait
/// - `sealed`: `AtomicBool`, status marker indicates the actor is mutably borrowed or not
/// - `type_id`: the type id of [`Acotr`](Actor) trait implementor
/// - `message_id`: the type id of the type implemented [`Message`](crate::message::Message) trait
/// - `name`: `String`, the name of the registered actor, the default format is `Acotr-{id}`,
/// customize it with [set_name](ActorRegister::set_name).
/// - `closed`: `AtomicBool`, status marker indicates the actor is closed or not
#[derive(Debug)]
pub struct ActorRegister {
    id: usize,
    name: String,
    type_id: TypeId,
    message_id: TypeId,
    inner: Arc<UnsafeCell<dyn Any>>,
    sealed: Arc<AtomicBool>,
    closed: AtomicBool,
}

/// An guardian created when an Actor gets registered
pub struct ActorGuard<A: Actor> {
    inner: Arc<UnsafeCell<A>>,
    sealed: Arc<AtomicBool>,
}

impl<A: Actor> fmt::Debug for ActorGuard<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActorGuard<_>")
            .field("inner", &"Arc<UnsafeCell<Actor>>")
            .field("sealed", &self.sealed)
            .finish()
    }
}

impl<A: Actor> Deref for ActorGuard<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        let ptr = Arc::as_ptr(&self.inner);
        let actor_cell: *mut A = UnsafeCell::raw_get(ptr as *const _);
        unsafe { transmute::<*mut A, &A>(actor_cell) }
    }
}

impl<A: Actor> DerefMut for ActorGuard<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = Arc::as_ptr(&self.inner);
        let actor_cell: *mut A = UnsafeCell::raw_get(ptr as *const _);
        unsafe { transmute::<*mut A, &mut A>(actor_cell) }
    }
}

/// **Safety**: since it is guardted with AtomicBool
/// for generic `A: Actor`
/// - nothing changes to the Inner data when downcasted to &A,
/// - seal the inner to deny more than one access when downcasted to &mut A
///
/// So it is safe to send &A between threads
unsafe impl Send for ActorRegister {}
/// **Safety**: since it is guardted with AtomicBool
/// for generic `A: Actor`
/// - nothing changes to the Inner data when downcasted to &A,
/// - seal the inner to deny more than one access when downcasted to &mut A
///
/// So it is safe to send &A between threads
unsafe impl Sync for ActorRegister {}

impl ActorRegister {
    /// create a Actor Register for an
    /// actor instance
    pub fn new<A>(act: A) -> Self
    where
        A: Actor,
    {
        let id = ACTORCOUNT.fetch_add(1, Ordering::Relaxed);
        Self {
            id,
            type_id: TypeId::of::<A>(),
            message_id: TypeId::of::<A::Message>(),
            name: format!("Actor-{}", id),
            inner: Arc::new(UnsafeCell::new(act)),
            sealed: Arc::new(AtomicBool::new(false)),
            closed: AtomicBool::new(false),
        }
    }

    /// the unqiue identity of the Actor
    pub fn id(&self) -> usize {
        self.id
    }

    /// the TypeId of the Actor
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// the TypeId of the [Actor::Message](crate::actor::Actor::Message)
    pub fn message_id(&self) -> TypeId {
        self.message_id
    }

    /// the name of the Actor
    /// you can set it with `set_name`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// set the name of the Actor
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    /// the state of the Actor
    /// you can set it with `set_closed`
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    /// set the actor is closed
    pub fn set_closed(&self, state: bool) {
        self.closed.store(state, Ordering::Relaxed);
    }

    /// indicator that whether the Actor is
    /// being downcasted to Actor and mutating
    pub fn is_sealed(&self) -> bool {
        self.sealed.load(Ordering::Relaxed)
    }

    /// downcast the type Actor type `&A`
    pub fn downcast_ref<A: Actor>(&self) -> Option<&A> {
        // make sure that the inner is being mutating and sealed
        // if ture, wait in a spin loop for completion
        while self.sealed.load(Ordering::Relaxed) {
            hint::spin_loop();
        }
        let ptr = self.inner.as_ref().get();
        let dyn_ptr = unsafe { transmute::<*mut dyn Any, &dyn Any>(ptr) };
        dyn_ptr.downcast_ref::<A>()
    }

    /// downcast the type Actor guard `ActorGuard`
    pub(crate) fn downcast_ref_cell<A: Actor>(&self) -> ActorGuard<A> {
        // make sure that the inner is being mutating and sealed
        // if ture, wait in a spin loop for completion
        while self.sealed.load(Ordering::Relaxed) {
            hint::spin_loop();
        }
        let ptr = Arc::as_ptr(&self.inner);
        let actor_cell: *mut A = UnsafeCell::raw_get(ptr as *const _);
        // **Safety**: as the inner data is guarded
        // with Arc, it is safe to downcast from
        // `Arc<UnsafeCell<A>>` to `Arc<A>`
        // as UnsafeCell<A> and A has the same size
        // and both are not mutable with Arc
        let actor_inner: *const UnsafeCell<_> =
            unsafe { transmute::<*mut A, *const UnsafeCell<A>>(actor_cell) };
        unsafe { Arc::increment_strong_count(actor_inner) };
        ActorGuard {
            //inner: Arc::new(UnsafeCell::new(actor_inner)),
            inner: unsafe { Arc::from_raw(actor_inner) },
            sealed: self.sealed.clone(),
        }
    }

    /// downcast the type Actor type `&mut A`
    /// then mutate it with the function `f`
    ///
    /// and return
    /// - `None`: downcast failed
    /// - `Some(false)`: downcasted and `f` return `Err(_)`
    /// - `Some(true)`: downcasted and `f` return `Ok(_)`
    ///
    /// the underlying actor will be
    /// guarded when downcasted
    /// no more mutable access is allowed during downcasted
    pub fn downcast_mut<A: Actor, F>(&self, mut f: F) -> Option<bool>
    where
        F: FnMut(&mut A) -> Result<(), ()>,
    {
        while self.sealed.load(Ordering::Acquire) {
            hint::spin_loop();
        }
        self.sealed.store(true, Ordering::Release);
        let ptr = self.inner.as_ref().get();
        let dyn_ptr = unsafe { transmute::<*mut dyn Any, &mut dyn Any>(ptr) };
        let inner = dyn_ptr.downcast_mut::<A>();
        match inner {
            None => None,
            Some(data) => match f(data) {
                Ok(_) => Some(true),
                Err(_) => Some(false),
            },
        }
    }
}

#[test]
fn test_downcast_cell() {
    extern crate alloc;
    use alloc::sync::Arc;
    use core::{cell::UnsafeCell, mem::transmute};

    struct Se {}

    let inner = Arc::new(UnsafeCell::new(Se {}));
    let _a = inner.clone();

    let ptr = Arc::as_ptr(&inner);
    let actor_cell: *mut Se = UnsafeCell::raw_get(ptr as *const _);
    let actor_inner: *const UnsafeCell<_> =
        unsafe { transmute::<*mut Se, *const UnsafeCell<Se>>(actor_cell) };

    let inn = unsafe { Arc::from_raw(actor_inner) };
    unsafe { Arc::increment_strong_count(actor_inner) };
    assert_eq!(Arc::strong_count(&inn), Arc::strong_count(&inner));
    assert_eq!(Arc::weak_count(&inn), Arc::weak_count(&inner));
    drop(_a);
    drop(inner);
    assert_eq!(Arc::strong_count(&inn), 1);
    assert_eq!(Arc::weak_count(&inn), 0);
}

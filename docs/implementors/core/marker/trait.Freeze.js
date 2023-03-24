(function() {var implementors = {
"crossbus":[["impl Freeze for <a class=\"enum\" href=\"crossbus/actor/enum.ActorState.html\" title=\"enum crossbus::actor::ActorState\">ActorState</a>",1,["crossbus::actor::ActorState"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/actor/enum.ActingState.html\" title=\"enum crossbus::actor::ActingState\">ActingState</a>",1,["crossbus::actor::ActingState"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/actor/struct.Handle.html\" title=\"struct crossbus::actor::Handle\">Handle</a>",1,["crossbus::actor::Handle"]],["impl&lt;A, F&gt; Freeze for <a class=\"struct\" href=\"crossbus/actor/struct.Localizer.html\" title=\"struct crossbus::actor::Localizer\">Localizer</a>&lt;A, F&gt;<span class=\"where fmt-newline\">where\n    F: Freeze,</span>",1,["crossbus::actor::Localizer"]],["impl&lt;M&gt; Freeze for <a class=\"struct\" href=\"crossbus/address/struct.Sender.html\" title=\"struct crossbus::address::Sender\">Sender</a>&lt;M&gt;",1,["crossbus::address::Sender"]],["impl&lt;M&gt; Freeze for <a class=\"struct\" href=\"crossbus/address/struct.WeakSender.html\" title=\"struct crossbus::address::WeakSender\">WeakSender</a>&lt;M&gt;",1,["crossbus::address::WeakSender"]],["impl&lt;M&gt; Freeze for <a class=\"struct\" href=\"crossbus/address/struct.Receiver.html\" title=\"struct crossbus::address::Receiver\">Receiver</a>&lt;M&gt;",1,["crossbus::address::Receiver"]],["impl&lt;M&gt; Freeze for <a class=\"struct\" href=\"crossbus/address/struct.WeakReceiver.html\" title=\"struct crossbus::address::WeakReceiver\">WeakReceiver</a>&lt;M&gt;",1,["crossbus::address::WeakReceiver"]],["impl&lt;M&gt; Freeze for <a class=\"struct\" href=\"crossbus/address/struct.Addr.html\" title=\"struct crossbus::address::Addr\">Addr</a>&lt;M&gt;",1,["crossbus::address::Addr"]],["impl&lt;T&gt; Freeze for <a class=\"enum\" href=\"crossbus/address/enum.QueueError.html\" title=\"enum crossbus::address::QueueError\">QueueError</a>&lt;T&gt;<span class=\"where fmt-newline\">where\n    T: Freeze,</span>",1,["crossbus::address::QueueError"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/blocker/enum.BlockingState.html\" title=\"enum crossbus::blocker::BlockingState\">BlockingState</a>",1,["crossbus::blocker::BlockingState"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/blocker/enum.BlockerState.html\" title=\"enum crossbus::blocker::BlockerState\">BlockerState</a>",1,["crossbus::blocker::BlockerState"]],["impl&lt;A&gt; Freeze for <a class=\"struct\" href=\"crossbus/blocker/struct.Blocker.html\" title=\"struct crossbus::blocker::Blocker\">Blocker</a>&lt;A&gt;",1,["crossbus::blocker::Blocker"]],["impl&lt;A&gt; Freeze for <a class=\"struct\" href=\"crossbus/context/struct.Context.html\" title=\"struct crossbus::context::Context\">Context</a>&lt;A&gt;",1,["crossbus::context::Context"]],["impl&lt;A&gt; Freeze for <a class=\"struct\" href=\"crossbus/context/struct.ContextRunner.html\" title=\"struct crossbus::context::ContextRunner\">ContextRunner</a>&lt;A&gt;",1,["crossbus::context::ContextRunner"]],["impl&lt;A&gt; Freeze for <a class=\"struct\" href=\"crossbus/delayer/struct.Delayer.html\" title=\"struct crossbus::delayer::Delayer\">Delayer</a>&lt;A&gt;<span class=\"where fmt-newline\">where\n    &lt;A as <a class=\"trait\" href=\"crossbus/actor/trait.Actor.html\" title=\"trait crossbus::actor::Actor\">Actor</a>&gt;::<a class=\"associatedtype\" href=\"crossbus/actor/trait.Actor.html#associatedtype.Message\" title=\"type crossbus::actor::Actor::Message\">Message</a>: Freeze,</span>",1,["crossbus::delayer::Delayer"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/delayer/enum.DelayingState.html\" title=\"enum crossbus::delayer::DelayingState\">DelayingState</a>",1,["crossbus::delayer::DelayingState"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/delayer/enum.DelayerState.html\" title=\"enum crossbus::delayer::DelayerState\">DelayerState</a>",1,["crossbus::delayer::DelayerState"]],["impl&lt;M&gt; Freeze for <a class=\"enum\" href=\"crossbus/delayer/enum.Indicator.html\" title=\"enum crossbus::delayer::Indicator\">Indicator</a>&lt;M&gt;<span class=\"where fmt-newline\">where\n    M: Freeze,</span>",1,["crossbus::delayer::Indicator"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/message/enum.MStreamingState.html\" title=\"enum crossbus::message::MStreamingState\">MStreamingState</a>",1,["crossbus::message::MStreamingState"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/message/enum.MStreamState.html\" title=\"enum crossbus::message::MStreamState\">MStreamState</a>",1,["crossbus::message::MStreamState"]],["impl&lt;S&gt; Freeze for <a class=\"struct\" href=\"crossbus/message/struct.MStreaming.html\" title=\"struct crossbus::message::MStreaming\">MStreaming</a>&lt;S&gt;<span class=\"where fmt-newline\">where\n    S: Freeze,</span>",1,["crossbus::message::MStreaming"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/reactor/struct.ReactorPair.html\" title=\"struct crossbus::reactor::ReactorPair\">ReactorPair</a>",1,["crossbus::reactor::ReactorPair"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/reactor/struct.ReactorHandle.html\" title=\"struct crossbus::reactor::ReactorHandle\">ReactorHandle</a>",1,["crossbus::reactor::ReactorHandle"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/reactor/enum.ReactingOrder.html\" title=\"enum crossbus::reactor::ReactingOrder\">ReactingOrder</a>",1,["crossbus::reactor::ReactingOrder"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/reactor/struct.Reactor.html\" title=\"struct crossbus::reactor::Reactor\">Reactor</a>",1,["crossbus::reactor::Reactor"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/register/struct.Register.html\" title=\"struct crossbus::register::Register\">Register</a>",1,["crossbus::register::Register"]],["impl !Freeze for <a class=\"struct\" href=\"crossbus/register/struct.ActorRegister.html\" title=\"struct crossbus::register::ActorRegister\">ActorRegister</a>",1,["crossbus::register::ActorRegister"]],["impl&lt;A&gt; Freeze for <a class=\"struct\" href=\"crossbus/register/struct.ActorGuard.html\" title=\"struct crossbus::register::ActorGuard\">ActorGuard</a>&lt;A&gt;",1,["crossbus::register::ActorGuard"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/rt/runtime_async_std/struct.Runtime.html\" title=\"struct crossbus::rt::runtime_async_std::Runtime\">Runtime</a>",1,["crossbus::rt::runtime_async_std::Runtime"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/rt/runtime_tokio/struct.Runtime.html\" title=\"struct crossbus::rt::runtime_tokio::Runtime\">Runtime</a>",1,["crossbus::rt::runtime_tokio::Runtime"]],["impl !Freeze for <a class=\"struct\" href=\"crossbus/rt/runtime_tokio/struct.TokioRuntime.html\" title=\"struct crossbus::rt::runtime_tokio::TokioRuntime\">TokioRuntime</a>",1,["crossbus::rt::runtime_tokio::TokioRuntime"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/rt/runtime_wasm32/struct.Runtime.html\" title=\"struct crossbus::rt::runtime_wasm32::Runtime\">Runtime</a>",1,["crossbus::rt::runtime_wasm32::Runtime"]],["impl Freeze for <a class=\"struct\" href=\"crossbus/rt/wasm_timeout/struct.WasmTimeOut.html\" title=\"struct crossbus::rt::wasm_timeout::WasmTimeOut\">WasmTimeOut</a>",1,["crossbus::rt::wasm_timeout::WasmTimeOut"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/stream/enum.StreamingState.html\" title=\"enum crossbus::stream::StreamingState\">StreamingState</a>",1,["crossbus::stream::StreamingState"]],["impl Freeze for <a class=\"enum\" href=\"crossbus/stream/enum.StreamState.html\" title=\"enum crossbus::stream::StreamState\">StreamState</a>",1,["crossbus::stream::StreamState"]],["impl&lt;A, S&gt; Freeze for <a class=\"struct\" href=\"crossbus/stream/struct.Streaming.html\" title=\"struct crossbus::stream::Streaming\">Streaming</a>&lt;A, S&gt;<span class=\"where fmt-newline\">where\n    S: Freeze,</span>",1,["crossbus::stream::Streaming"]]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()
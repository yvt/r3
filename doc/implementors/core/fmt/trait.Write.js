(function() {var implementors = {};
implementors["embedded_hal"] = [{"text":"impl&lt;Word, Error&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Write.html\" title=\"trait core::fmt::Write\">Write</a> for dyn <a class=\"trait\" href=\"embedded_hal/serial/trait.Write.html\" title=\"trait embedded_hal::serial::Write\">Write</a>&lt;Word, Error = Error&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;Word: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;u8&gt;,&nbsp;</span>","synthetic":false,"types":["embedded_hal::serial::Write"]}];
implementors["staticvec"] = [{"text":"impl&lt;const N:&nbsp;usize&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Write.html\" title=\"trait core::fmt::Write\">Write</a> for <a class=\"struct\" href=\"staticvec/struct.StaticString.html\" title=\"struct staticvec::StaticString\">StaticString</a>&lt;N&gt;","synthetic":false,"types":["staticvec::string::StaticString"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()
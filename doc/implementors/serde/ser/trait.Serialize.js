(function() {var implementors = {};
implementors["multimap"] = [{"text":"impl&lt;K, V, BS&gt; Serialize for MultiMap&lt;K, V, BS&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;K: Serialize + Eq + Hash,<br>&nbsp;&nbsp;&nbsp;&nbsp;V: Serialize,<br>&nbsp;&nbsp;&nbsp;&nbsp;BS: BuildHasher,&nbsp;</span>","synthetic":false,"types":[]}];
implementors["serde_json"] = [{"text":"impl Serialize for Map&lt;String, Value&gt;","synthetic":false,"types":[]},{"text":"impl Serialize for Value","synthetic":false,"types":[]},{"text":"impl Serialize for Number","synthetic":false,"types":[]}];
implementors["tracing_serde"] = [{"text":"impl&lt;'a&gt; Serialize for SerializeFieldMap&lt;'a, Event&lt;'_&gt;&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeFieldMap&lt;'a, Attributes&lt;'_&gt;&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeFieldMap&lt;'a, Record&lt;'_&gt;&gt;","synthetic":false,"types":[]},{"text":"impl Serialize for SerializeField","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeFieldSet&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeLevel&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeId&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeMetadata&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeEvent&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeAttributes&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Serialize for SerializeRecord&lt;'a&gt;","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()
(function() {var implementors = {};
implementors["beef"] = [{"text":"impl&lt;T:&nbsp;?Sized, U&gt; Borrow&lt;T&gt; for Cow&lt;'_, T, U&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Beef,<br>&nbsp;&nbsp;&nbsp;&nbsp;U: Capacity,&nbsp;</span>","synthetic":false,"types":[]}];
implementors["smol_str"] = [{"text":"impl Borrow&lt;str&gt; for SmolStr","synthetic":false,"types":[]}];
implementors["tinyvec"] = [{"text":"impl&lt;A:&nbsp;Array&gt; Borrow&lt;[&lt;A as Array&gt;::Item]&gt; for ArrayVec&lt;A&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'s, T&gt; Borrow&lt;[T]&gt; for SliceVec&lt;'s, T&gt;","synthetic":false,"types":[]},{"text":"impl&lt;A:&nbsp;Array&gt; Borrow&lt;[&lt;A as Array&gt;::Item]&gt; for TinyVec&lt;A&gt;","synthetic":false,"types":[]}];
implementors["vec1"] = [{"text":"impl&lt;T&gt; Borrow&lt;[T]&gt; for Vec1&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl&lt;T&gt; Borrow&lt;Vec&lt;T, Global&gt;&gt; for Vec1&lt;T&gt;","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()
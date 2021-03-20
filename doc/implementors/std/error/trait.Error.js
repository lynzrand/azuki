(function() {var implementors = {};
implementors["clap"] = [{"text":"impl Error for Error","synthetic":false,"types":[]}];
implementors["combine"] = [{"text":"impl Error for UnexpectedParse","synthetic":false,"types":[]},{"text":"impl Error for StringStreamError","synthetic":false,"types":[]},{"text":"impl&lt;T, R, P&gt; Error for Errors&lt;T, R, P&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;P: Display + Debug,<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Display + Debug,<br>&nbsp;&nbsp;&nbsp;&nbsp;R: Display + Debug,&nbsp;</span>","synthetic":false,"types":[]},{"text":"impl&lt;E, P&gt; Error for Error&lt;E, P&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;E: Error,<br>&nbsp;&nbsp;&nbsp;&nbsp;P: Display + Debug,&nbsp;</span>","synthetic":false,"types":[]}];
implementors["num_bigint"] = [{"text":"impl Error for ParseBigIntError","synthetic":false,"types":[]},{"text":"impl&lt;T&gt; Error for TryFromBigIntError&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Debug,&nbsp;</span>","synthetic":false,"types":[]}];
implementors["num_complex"] = [{"text":"impl&lt;E:&nbsp;Error&gt; Error for ParseComplexError&lt;E&gt;","synthetic":false,"types":[]}];
implementors["num_rational"] = [{"text":"impl Error for ParseRatioError","synthetic":false,"types":[]}];
implementors["os_str_bytes"] = [{"text":"impl Error for EncodingError","synthetic":false,"types":[]}];
implementors["proc_macro2"] = [{"text":"impl Error for LexError","synthetic":false,"types":[]}];
implementors["regex_syntax"] = [{"text":"impl Error for Error","synthetic":false,"types":[]},{"text":"impl Error for Error","synthetic":false,"types":[]},{"text":"impl Error for Error","synthetic":false,"types":[]},{"text":"impl Error for CaseFoldError","synthetic":false,"types":[]},{"text":"impl Error for UnicodeWordError","synthetic":false,"types":[]}];
implementors["strsim"] = [{"text":"impl Error for StrSimError","synthetic":false,"types":[]}];
implementors["syn"] = [{"text":"impl Error for Error","synthetic":false,"types":[]}];
implementors["termcolor"] = [{"text":"impl Error for ParseColorError","synthetic":false,"types":[]}];
implementors["vec1"] = [{"text":"impl Error for Size0Error","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()
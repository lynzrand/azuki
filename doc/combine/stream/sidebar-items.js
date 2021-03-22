initSidebarItems({"fn":[["decode","Decodes `input` using `parser`."],["uncons",""],["uncons_range",""],["uncons_while","Removes items from the input while `predicate` returns `true`."],["uncons_while1","Takes items from stream, testing each one with `predicate` returns a range of at least one items which passed `predicate`."]],"mod":[["buf_reader",""],["buffered","Stream wrapper which provides a `ResetStream` impl for `StreamOnce` impls which do not have one."],["decoder",""],["easy","Stream wrapper which provides an informative and easy to use error type."],["position","Stream wrapper which provides more detailed position information."],["read","Stream wrapper allowing `std::io::Read` to be used"],["span",""],["state","Stream wrapper allowing custom state to be used."]],"struct":[["CompleteStream","Stream type which indicates that the stream is complete if end of input is reached"],["IteratorStream","Wrapper around iterators which allows them to be treated as a stream. Returned by [`IteratorStream::new`]."],["MaybePartialStream",""],["PartialStream","Stream type which indicates that the stream is partial if end of input is reached"],["PointerOffset","Newtype around a pointer offset into a slice stream (`&[T]`/`&str`)."],["SliceStream","Newtype for constructing a stream from a slice where the items in the slice are not copyable."]],"trait":[["Positioned","A type which has a position."],["Range","Trait representing a range of elements."],["RangeStream","A `RangeStream` is an extension of `Stream` which allows for zero copy parsing."],["RangeStreamOnce","A `RangeStream` is an extension of `StreamOnce` which allows for zero copy parsing."],["ResetStream","A `StreamOnce` which can create checkpoints which the stream can be reset to"],["Stream","A stream of tokens which can be duplicated"],["StreamOnce","`StreamOnce` represents a sequence of items that can be extracted one by one."]],"type":[["StreamErrorFor","Convenience alias over the `StreamError` for the input stream `Input`"]]});
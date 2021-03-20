initSidebarItems({"fn":[["_ty",""],["basic_blocks",""],["bb_id",""],["binary_instruction",""],["binary_op",""],["bool_ty",""],["branch_or_branch_if_jump_instruction",""],["comma_sep_list","Parse a comma-separated list. The internal parser should skip spaces."],["dec_number",""],["func_call_instruction",""],["func_header",""],["func_ty",""],["hex_number",""],["ident",""],["instruction",""],["int_ty",""],["jump_instructions",""],["nl1","Matches some spaces, a new line, and some other spaces or newlines"],["number",""],["param_instruction",""],["parse_func",""],["parse_program",""],["phi_instruction",""],["return_jump_instruction",""],["single_basic_block",""],["spaces0","Matches zero or more non-newline space characters"],["spaces1","Matches one or more non-newline space characters, or the end of a line"],["ty",""],["unit_ty",""],["unreachable_jump_instruction",""],["unsigned_dec_number",""],["value",""],["value_instruction",""],["variable",""]],"mod":[["easy_parse","Stream wrapper which provides an informative and easy to use error type."],["parse_stream","Streams are similar to the `Iterator` trait in that they represent some sequential set of items which can be retrieved one by one. Where `Stream`s differ is that they are allowed to return errors instead of just `None` and if they implement the `RangeStreamOnce` trait they are also capable of returning multiple items at the same time, usually in the form of a slice."]],"struct":[["VariableNamingCtx",""]],"trait":[["EasyParser","Provides the `easy_parse` method which provides good error messages by default"],["Parser","By implementing the `Parser` trait a type says that it can be used to parse an input stream into the type `Output`."],["Stream","A stream of tokens which can be duplicated"]]});
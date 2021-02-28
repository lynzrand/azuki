grammar AzukiTac;

WS: [ \t]+ -> skip;

// Basic parts
fragment Digit: [0-9];
fragment Letter: [a-zA-Z];
fragment HexDigit: Digit | [a-fA-F];
fragment AlphaNum: Digit | Letter;
fragment IdentChar: AlphaNum | '_' | '$';

Number: Digit+;
Ident: IdentChar+;
HexNumber: HexDigit+;

Variable: '%' Number;
GlobalVariable: '@' Ident;
NumberLiteral: '#' (('+' | '-')? Number | '0x' HexNumber);
BasicBlock: 'bb' Number;
DiscardVariable: '%_';

// types
IntegerType: 'i' Number;
BooleanType: 'b' Number;
UnitType: 'unit';

// keywords
FN: 'fn';
GLOBAL: 'global';

// Operators
ADD: 'add';
SUB: 'sub';
MUL: 'mul';
DIV: 'div';

GT: 'gt';
GE: 'ge';
LT: 'lt';
LE: 'le';
EQ: 'eq';
NE: 'ne';

PHI: 'phi';
BRANCH: 'br';
IF: 'if';
UNREACHABLE: 'unreachable';
RETURN: 'return';
CALL: 'call';

// misc
LINEFEED: '\n';

// Grammar components
literal: NumberLiteral;
value: Variable | GlobalVariable | NumberLiteral;

int_ty: IntegerType;
bool_ty: BooleanType;
unit_ty: UnitType;
ptr_ty: '*' ty;
func_ty: FN function_param '->' ty;
ty: int_ty | bool_ty | unit_ty | ptr_ty | func_ty;

// instructions
binary_op: ADD | SUB | MUL | DIV | GT | GE | LT | LE | EQ | NE;
binary_inst: value binary_op value;

fn_param_list: (value (',' value)*)?;
fn_call_inst: CALL GlobalVariable '(' fn_param_list ')';

phi_source: '(' Variable ',' BasicBlock ')';
phi_inst: PHI '[' (phi_source (',' phi_source)*)? ']';

val_inst: value;

variable: Variable;
inst_lhs: ty variable | DiscardVariable;
inst_rhs: binary_inst | phi_inst | val_inst | fn_call_inst;
inst: inst_lhs '=' ty inst_rhs LINEFEED;

unreachable_inst: UNREACHABLE;
uncond_branch_inst: BRANCH BasicBlock;
cond_branch_inst: BRANCH BasicBlock IF value;
return_inst: RETURN value;
jump_inst:
	(
		unreachable_inst
		| uncond_branch_inst
		| cond_branch_inst
		| return_inst
	) LINEFEED;

// basic block
basic_block_id: BasicBlock ':';
basic_block: basic_block_id LINEFEED inst* jump_inst+;

// function
function_param: '(' (ty (',' ty)*)? ')';
function:
	FN GlobalVariable function_param '->' ty '{' LINEFEED basic_block+ '}' LINEFEED;

// program
global_var: GLOBAL GlobalVariable '=' literal LINEFEED;
program: (global_var | function)*;

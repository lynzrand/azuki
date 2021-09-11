# Azuki

**Development of Azuki has been suspended. Please head over to [miniSysY](https://github.com/BUAA-SE-Compiling/miniSysY) and [miniSysY-tutorial](https://github.com/BUAA-SE-Compiling/miniSysY-tutorial) for updates on BUAA-SE's compiler course updates.**

Azuki is an experimental lab experiment design & reference implementation for Compilers Design course. This project also serves as a part of Rynco Maekawa's bachelor's degree thesis.

## Features

- [x] Support for a C-like language (C0)
  - [x] Lexing
  - [x] Parsing
  - [x] AST generation
- [x] Generate SSA Intermediate Representation from AST
- [ ] Perform optimization in IR
  - [ ] TODO: List optimizations here
- [x] Run IR inside a virtual machine
- [ ] Handbook for students using this project
- [ ] Grading tools and test cases

## Project structure

```
./                      root folder
    docs/               documents & handbooks
    bindings/           bindings for different languages
    crates/             libraries used in Azuki
        syntax/         lexing & parsing & AST
        tac/            three-address code (TAC) definition
        tacgen/         TAC code generation
        opt/            optimization passes
        vm/             virtual machine
    src/                reference design source
```


## Related projects

Online Judger (the other half of thesis project): [Rurikawa](https://github.com/BUAA-SE-Compiling/rurikawa)

Previous attempts of Compilers Design lab in The College of Software in BUAA: 

- [Natrium](https://github.com/BUAA-SE-Compiling/natrium)
- [C0](https://github.com/BUAA-SE-Compiling/c0-handbook)


## License

Azuki is licensed under MIT license.

Copyright (c) 2020 Rynco Maekawa.

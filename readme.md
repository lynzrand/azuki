# Azuki

Azuki is a experimental lab experiment for Compilers Design course. This project serves as a part of Rynco Maekawa's bachelor's degree thesis.

## Project structure

This is a *planned* structure of this project.

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

Auto-grading system (online judge): [Rurikawa](https://github.com/BUAA-SE-Compiling/rurikawa)

Previous attempt of this lab experiment: [Natrium](https://github.com/BUAA-SE-Compiling/natrium)


## License

Azuki is licensed under MIT license.

Copyright (c) 2020 Rynco Maekawa.

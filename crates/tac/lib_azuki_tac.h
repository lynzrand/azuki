#ifndef LIB_AZUKI_TAC_H
#define LIB_AZUKI_TAC_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define Azuki_PTR_SIZE 8

typedef struct Azuki_Inst Azuki_Inst;

/**
 * A function made of TAC instructions.
 *
 * The instructions are represented as an indirect doubly linked list inside the
 * `arena` using item indices. Every basic block holds the start and end index
 * of its instructions.
 */
typedef struct Azuki_TacFunc Azuki_TacFunc;

/**
 * A representation of basic type that has O(1) clone and sizes no more than
 * `2 * sizeof(usize)`.
 *
 * > I know this is worse than using an external type repository, but hey you
 * > can directly compare these!
 */
typedef struct Azuki_Ty Azuki_Ty;

struct Azuki_TacFunc *AzukiTacFunc_NewUntyped(const int8_t *name);

struct Azuki_TacFunc *AzukiTacFunc_New(const int8_t *name, struct Azuki_Ty *ty);

uint64_t AzukiTacFunc_AddInst(struct Azuki_TacFunc *func, struct Azuki_Inst *inst, uintptr_t bb);

void AzukiTacFunc_Destructor(struct Azuki_TacFunc *func);

#endif /* LIB_AZUKI_TAC_H */

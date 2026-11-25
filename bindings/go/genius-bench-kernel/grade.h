#ifndef GENIUS_BENCH_GO_GRADE_H
#define GENIUS_BENCH_GO_GRADE_H

#include <stddef.h>
#include <stdint.h>

typedef struct { const char *data; size_t len; } gb_string;
typedef struct { const gb_string *data; size_t len; } gb_strings;
typedef struct { gb_string path, contents; } gb_file;
typedef struct { const gb_file *data; size_t len; } gb_files;
enum { GB_NULL, GB_FALSE, GB_TRUE };
typedef struct { gb_string name; uint8_t value; } gb_fact;
typedef struct { const gb_fact *data; size_t len; } gb_facts;

typedef struct {
    gb_string id, prompt;
    gb_files fixture, edits;
    gb_strings present_path, allowed_files;
    uint8_t completion;
} gb_task_input;

enum { GB_UNCHECKED, GB_PASSED, GB_FAILED, GB_INVALID_SUBMISSION, GB_TIMED_OUT, GB_HARNESS_ERROR };
typedef struct { uint8_t tag; gb_string detail; } gb_verification;
typedef struct {
    gb_strings changed_files;
    gb_verification verification;
    uint8_t response;
    gb_facts facts;
} gb_outcome;

typedef struct gb_task gb_task;
typedef struct gb_result gb_result;
enum { GB_SCORES, GB_VALIDATION, GB_GRADING, GB_INVALID_INPUT };
typedef struct { uint8_t tag; size_t count; gb_string code, detail; } gb_result_info;
typedef struct { gb_string name, message; uint8_t passed; } gb_row;

gb_result *genius_bench_go_new_task(const gb_task_input *input, gb_task **task);
void genius_bench_go_free_task(gb_task *task);
gb_result *genius_bench_go_grade(const gb_task *task, const gb_outcome *outcome);
gb_result *genius_bench_go_grade_reference(const gb_task *task, gb_verification verification);
gb_result_info genius_bench_go_info(const gb_result *result);
gb_row genius_bench_go_get_row(const gb_result *result, size_t index);
void genius_bench_go_free_result(gb_result *result);

#endif

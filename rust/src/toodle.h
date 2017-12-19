#include <stdint.h>
#include "labels.h"

struct Toodle;

struct Toodle* new_toodle(const char* uri);

void toodle_destroy(struct Toodle* toodle);

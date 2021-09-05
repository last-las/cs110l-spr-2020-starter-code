1. vec->capacity is always zero.
2. didn't free the old pointer in vec_push when vec->length equals vec->capacity.
3. in vec_free() should free vec->data first before vec itself is freed.
4. double free in main.

### function sha1(hash_out pointer, const char *str, uint32_t len)
- creates a sha1_context structure : ctx.
- sha1Init(&ctx)
- iterating till for (ii=0; ii<len; ii+=1)
	- SHA1Update(&ctx, (const unsigned char*)str + ii, 1);


### sha1_ctx structure
```
    uint32_t state[5];
    uint32_t count[2];
    unsigned char buffer[64];
```

### function sha1init( sha1_ctx * context)
```
//Initializing constants
    context->state[0] = 0x67452301;
    context->state[1] = 0xEFCDAB89;
    context->state[2] = 0x98BADCFE;
    context->state[3] = 0x10325476;
    context->state[4] = 0xC3D2E1F0;
    context->count[0] = context->count[1] = 0;
```

### function sha1_update(sha1_ctx * context, const unsigned char *data, uint32_t len)
- This block maintains a 64 bit counter, context->count stores total number of bits processed, split into 2 32-bit integers. count[0] is the lower 32 bits, and count[1] in the upper 32 bits, 
- shifting converts bytes to bits, len << 3  ==  len * 8, coz sha1 keeps track of the bits, we need the number of bits, and if there is any overflow, it is added in count[1], since count[0] is only 32 bits, it overflows...so it might wrap itself, now for the upper bits, i.e the greater half of the 64 bit counter, we do len >> 29 this checks how much more do we exceed from the 32 bits, and hence adds to the upper 32 bits of the counter. j & 63 = 130 % 64 = 2

```
    j = context->count[0];
    if ((context->count[0] += len << 3) < j)
        context->count[1]++;
    context->count[1] += (len >> 29);
    j = (j >> 3) & 63; //updating the buffer index
    if ((j + len) > 63) //check if new data will fill in the block,
    {
        memcpy(&context->buffer[j], data, (i = 64 - j));
        SHA1Transform(context->state, context->buffer);
        for (; i + 63 < len; i += 64)
        {
            SHA1Transform(context->state, &data[i]);
        }
        j = 0;
    }
    else
        i = 0;
    memcpy(&context->buffer[j], &data[i], len - i);
```

### function sha1transform( uint32_t state[5],const unsigned char buffer[64])

creating a shared memory of 64 bytes with union which creates, same memory, two interpretations:
- Byte-wise view (`unsigned char`)
- 32-bit word view (`uint32_t`)
SHA-1 processes data in **512-bit blocks (64 bytes)**, but internally it works on: **16 words of 32 bits each**, Instead of copying and converting manually, this union lets you **reinterpret the same memory**.
a 512 bit block can be split in many ways
- 16 × 32-bit
- 32 × 16-bit
- 64 × 8-bit
but SHA-1 **chooses 16 × 32-bit** for very specific reasons.
SHA-1’s core operations are defined on **32-bit words**:

- Bitwise operations (`AND`, `OR`, `XOR`)
- Additions modulo 2^32
- Circular left rotations (very important)

All variables (`a, b, c, d, e, w[i]`) are **32-bit values**
If you switch to 16-bit words:
- You’d need different rotation sizes
- Different overflow behavior
- Different constants
- Entire algorithm changes

There are 2 modes defined a safe and a slower approach another is a faster but yet riskier one, 
- safer version if SHA1HANDSOFF is defined, Allocate a union. Copy 64 bytes from `buffer` into it, because of union we have, `block->c` = raw bytes and if`block->l` = same data as 32-bit integers.
- fast but risky version, Treats `buffer` directly as a `CHAR64LONG16`. No copying here, `buffer` (bytes) is directly viewed as: `block->c`, `block->l`.`buffer` is `const unsigned char *`. But we cast it to a union pointer. That union allows writing → violates const safety. **Strict aliasing rules** may be violated. Can cause undefined behavior on some compilers.
```
    uint32_t a, b, c, d, e;

    typedef union
    {
        unsigned char c[64];
        uint32_t l[16];
    } CHAR64LONG16;

#ifdef SHA1HANDSOFF
    CHAR64LONG16 block[1];      /* use array to appear as a pointer */

    memcpy(block, buffer, 64);
#else
    /* The following had better never be used because it causes the
     * pointer-to-const buffer to be cast into a pointer to non-const.
     * And the result is written through.  I threw a "const" in, hoping
     * this will cause a diagnostic.
     */
    CHAR64LONG16 *block = (const CHAR64LONG16 *) buffer;
#endif
```


> rust addon In Rust, some operations can **fail**, so instead of returning a value directly, they return a wrapper like:

- `Option<T>` → might be `Some(value)` or `None`
- `Result<T, E>` → might be `Ok(value)` or `Err(error)`

👉 `unwrap()` is a shortcut that says:

> “Give me the value inside. If there isn’t one, crash the program.”



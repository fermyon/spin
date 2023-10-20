#ifndef __POLYFILL_VLD1Q_S8_X4__
#define __POLYFILL_VLD1Q_S8_X4__

inline int8x16x4_t vld1q_s8_x4(const int8_t *p)
{
    int8x16x4_t ret;
    ret.val[0] = vld1q_s8(p + 0);
    ret.val[1] = vld1q_s8(p + 16);
    ret.val[2] = vld1q_s8(p + 32);
    ret.val[3] = vld1q_s8(p + 48);
    return ret;
}

#endif // __POLYFILL_VLD1Q_S8_X4__

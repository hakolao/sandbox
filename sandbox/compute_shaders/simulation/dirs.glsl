#define UP_LEFT 0
#define UP 1
#define UP_RIGHT 2
#define RIGHT 3
#define DOWN_RIGHT 4
#define DOWN 5
#define DOWN_LEFT 6
#define LEFT 7

const ivec2 OFFSETS[8] = ivec2[8](ivec2(-1, 1), ivec2(0, 1), ivec2(1, 1), ivec2(1, 0),
ivec2(1, -1), ivec2(0, -1), ivec2(-1, -1), ivec2(-1, 0));
#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdarg.h>
#include <time.h>
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>

#ifdef _WIN32
#include <windows.h>
#include <direct.h>
#include <io.h>
#include <sys/types.h>
#include <sys/stat.h>
#define PATH_SEP '\\'
#define PATH_SEP_STR "\\"
#define access _access
#define mkdir _mkdir
#define F_OK 0
#define W_OK 2
#define R_OK 4
#define stat _stat
#define S_IFDIR _S_IFDIR
#define S_IFMT _S_IFMT
#define S_ISDIR(m) (((m) & _S_IFMT) == _S_IFDIR)
#else
#include <unistd.h>
#include <dirent.h>
#include <sys/types.h>
#include <sys/stat.h>
#define PATH_SEP '/'
#define PATH_SEP_STR "/"
#define S_ISDIR(m) (((m) & S_IFMT) == S_IFDIR)
#endif

#include <zlib.h>

#define GVT_VERSION "0.1.0"
#define SHA1_DIGEST_LENGTH 20
#define MAX_PATH_LEN 4096
#define MAX_MSG_LEN 4096

typedef enum {
    YUYAN_UNKNOWN = 0,
    YUYAN_GVTCIER,
    YUYAN_HOLYC,
    YUYAN_C,
    YUYAN_CPP,
    YUYAN_PYTHON,
    YUYAN_RUST,
    YUYAN_JAVASCRIPT,
    YUYAN_JAVA,
    YUYAN_ASSEMBLY,
    YUYAN_HTML,
    YUYAN_GO,
    YUYAN_ZIG,
    YUYAN_FORTRAN,
    YUYAN_OBJECTIVE_C,
    YUYAN_TYPESCRIPT,
    YUYAN_RUBY,
    YUYAN_PHP,
    YUYAN_PERL,
    YUYAN_LUA,
    YUYAN_SHELL,
    YUYAN_POWERSHELL,
    YUYAN_BATCH,
    YUYAN_TCL,
    YUYAN_KOTLIN,
    YUYAN_CSS,
    YUYAN_VUE,
    YUYAN_SVELTE,
    YUYAN_JSX,
    YUYAN_TEXT
} yuyan_leixing_t;

typedef struct {
    const char* kuozhan;
    yuyan_leixing_t yuyan;
    const char* mingcheng;
    const char* yanse;
} kuozhan_mapping_t;

static kuozhan_mapping_t kuozhan_biao[] = {
    {".gc", YUYAN_GVTCIER, "Gvtcier", "\033[1;33m"},
    {".hc", YUYAN_HOLYC, "HolyC", "\033[1;35m"},
    {".c", YUYAN_C, "C", "\033[1;34m"},
    {".h", YUYAN_C, "C", "\033[1;34m"},
    {".cpp", YUYAN_CPP, "C++", "\033[1;35m"},
    {".hpp", YUYAN_CPP, "C++", "\033[1;35m"},
    {".cxx", YUYAN_CPP, "C++", "\033[1;35m"},
    {".cc", YUYAN_CPP, "C++", "\033[1;35m"},
    {".py", YUYAN_PYTHON, "Python", "\033[1;33m"},
    {".pyw", YUYAN_PYTHON, "Python", "\033[1;33m"},
    {".pyi", YUYAN_PYTHON, "Python", "\033[1;33m"},
    {".rs", YUYAN_RUST, "Rust", "\033[1;31m"},
    {".js", YUYAN_JAVASCRIPT, "JavaScript", "\033[1;32m"},
    {".mjs", YUYAN_JAVASCRIPT, "JavaScript", "\033[1;32m"},
    {".cjs", YUYAN_JAVASCRIPT, "JavaScript", "\033[1;32m"},
    {".java", YUYAN_JAVA, "Java", "\033[1;31m"},
    {".asm", YUYAN_ASSEMBLY, "Assembly", "\033[1;36m"},
    {".s", YUYAN_ASSEMBLY, "Assembly", "\033[1;36m"},
    {".S", YUYAN_ASSEMBLY, "Assembly", "\033[1;36m"},
    {".html", YUYAN_HTML, "HTML", "\033[1;36m"},
    {".htm", YUYAN_HTML, "HTML", "\033[1;36m"},
    {".go", YUYAN_GO, "Go", "\033[1;34m"},
    {".zig", YUYAN_ZIG, "Zig", "\033[1;33m"},
    {".f", YUYAN_FORTRAN, "Fortran", "\033[1;32m"},
    {".f90", YUYAN_FORTRAN, "Fortran", "\033[1;32m"},
    {".f95", YUYAN_FORTRAN, "Fortran", "\033[1;32m"},
    {".m", YUYAN_OBJECTIVE_C, "Objective-C", "\033[1;34m"},
    {".mm", YUYAN_OBJECTIVE_C, "Objective-C", "\033[1;34m"},
    {".ts", YUYAN_TYPESCRIPT, "TypeScript", "\033[1;34m"},
    {".tsx", YUYAN_TYPESCRIPT, "TypeScript", "\033[1;34m"},
    {".rb", YUYAN_RUBY, "Ruby", "\033[1;31m"},
    {".php", YUYAN_PHP, "PHP", "\033[1;35m"},
    {".pl", YUYAN_PERL, "Perl", "\033[1;36m"},
    {".pm", YUYAN_PERL, "Perl", "\033[1;36m"},
    {".lua", YUYAN_LUA, "Lua", "\033[1;34m"},
    {".sh", YUYAN_SHELL, "Shell", "\033[1;32m"},
    {".bash", YUYAN_SHELL, "Bash", "\033[1;32m"},
    {".zsh", YUYAN_SHELL, "Zsh", "\033[1;32m"},
    {".fish", YUYAN_SHELL, "Fish", "\033[1;32m"},
    {".ps1", YUYAN_POWERSHELL, "PowerShell", "\033[1;36m"},
    {".bat", YUYAN_BATCH, "Batch", "\033[1;37m"},
    {".cmd", YUYAN_BATCH, "Batch", "\033[1;37m"},
    {".tcl", YUYAN_TCL, "TCL", "\033[1;33m"},
    {".kt", YUYAN_KOTLIN, "Kotlin", "\033[1;35m"},
    {".kts", YUYAN_KOTLIN, "Kotlin", "\033[1;35m"},
    {".css", YUYAN_CSS, "CSS", "\033[1;36m"},
    {".scss", YUYAN_CSS, "SCSS", "\033[1;36m"},
    {".sass", YUYAN_CSS, "Sass", "\033[1;36m"},
    {".less", YUYAN_CSS, "Less", "\033[1;36m"},
    {".vue", YUYAN_VUE, "Vue", "\033[1;32m"},
    {".svelte", YUYAN_SVELTE, "Svelte", "\033[1;33m"},
    {".jsx", YUYAN_JSX, "JSX", "\033[1;32m"},
    {".txt", YUYAN_TEXT, "Text", "\033[0m"},
    {".log", YUYAN_TEXT, "Text", "\033[0m"},
    {".md", YUYAN_TEXT, "Markdown", "\033[0m"},
    {NULL, YUYAN_UNKNOWN, "Unknown", "\033[0m"}
};

typedef enum {
    GVT_OK = 0,
    GVT_ERR_NOT_REPO,
    GVT_ERR_FILE_NOT_FOUND,
    GVT_ERR_IO,
    GVT_ERR_SHA1,
    GVT_ERR_ZLIB,
    GVT_ERR_CORRUPT,
    GVT_ERR_UNCOMMITTED,
    GVT_ERR_MEMORY,
    GVT_ERR_INVALID_COMMIT,
    GVT_ERR_BRANCH_EXISTS,
    GVT_ERR_ALREADY_REPO
} gvt_error_t;

typedef enum {
    OBJ_WENJIAN,
    OBJ_MULU,
    OBJ_TIJIAO,
    OBJ_BIAOQIAN
} duixiang_leixing_t;

typedef struct {
    uint8_t shaxun[20];
    duixiang_leixing_t leixing;
    void* shuju;
    size_t daxiao;
} duixiang_t;

typedef struct {
    uint32_t chuangjian_shijian;
    uint32_t chuangjian_naimiao;
    uint32_t xiugai_shijian;
    uint32_t xiugai_naimiao;
    uint32_t shebei;
    uint32_t jiedian;
    uint32_t quanxian;
    uint32_t yonghu_id;
    uint32_t zu_id;
    uint32_t wenjian_daxiao;
    uint8_t shaxun[20];
    uint16_t lujing_changdu;
    char lujing[1];
} suoyin_tiaomu_t;

typedef struct {
    uint32_t tiaomu_shuliang;
    suoyin_tiaomu_t** tiaomu;
} suoyin_t;

typedef struct {
    uint8_t mulu_shaxun[20];
    uint8_t fu_tijiao[20];
    char zuozhe[256];
    char youxiang[256];
    time_t shijianchuo;
    int shiqu_pianyi;
    char xiaoxi[1024];
} tijiao_t;

static const char* gvt_cuowu_zifuchuan(gvt_error_t e) {
    switch(e) {
        case GVT_OK: return "成功";
        case GVT_ERR_NOT_REPO: return "不是Gvt仓库";
        case GVT_ERR_FILE_NOT_FOUND: return "文件不存在";
        case GVT_ERR_IO: return "IO错误";
        case GVT_ERR_SHA1: return "SHA-1错误";
        case GVT_ERR_ZLIB: return "压缩错误";
        case GVT_ERR_CORRUPT: return "对象损坏";
        case GVT_ERR_UNCOMMITTED: return "有未提交变更";
        case GVT_ERR_MEMORY: return "内存不足";
        case GVT_ERR_INVALID_COMMIT: return "无效提交";
        case GVT_ERR_BRANCH_EXISTS: return "分支已存在";
        case GVT_ERR_ALREADY_REPO: return "已是Gvt仓库";
        default: return "未知错误";
    }
}

static void sha1_jisuan(const unsigned char* shuju, size_t changdu, uint8_t shaxun[20]) {
    uint32_t h0 = 0x67452301, h1 = 0xEFCDAB89, h2 = 0x98BADCFE, h3 = 0x10325476, h4 = 0xC3D2E1F0;
    uint32_t a, b, c, d, e, f, k, temp;
    uint32_t w[80];
    size_t i, j;
    size_t msg_len = changdu + 1 + 8;
    size_t pad_len = 64 - (msg_len % 64);
    if (pad_len < 8) pad_len += 64;
    msg_len = changdu + 1 + pad_len + 8;
    unsigned char* msg = malloc(msg_len);
    if (!msg) return;
    memset(msg, 0, msg_len);
    memcpy(msg, shuju, changdu);
    msg[changdu] = 0x80;
    uint64_t bit_len = (uint64_t)changdu * 8;
    for (i = 0; i < 8; i++) {
        msg[changdu + 1 + pad_len + i] = (bit_len >> (56 - i * 8)) & 0xFF;
    }
    for (i = 0; i < msg_len; i += 64) {
        for (j = 0; j < 16; j++) {
            w[j] = (msg[i + j * 4] << 24) | (msg[i + j * 4 + 1] << 16) |
                   (msg[i + j * 4 + 2] << 8) | msg[i + j * 4 + 3];
        }
        for (j = 16; j < 80; j++) {
            w[j] = (w[j - 3] ^ w[j - 8] ^ w[j - 14] ^ w[j - 16]);
            w[j] = (w[j] << 1) | (w[j] >> 31);
        }
        a = h0; b = h1; c = h2; d = h3; e = h4;
        for (j = 0; j < 80; j++) {
            if (j < 20) {
                f = (b & c) | ((~b) & d);
                k = 0x5A827999;
            } else if (j < 40) {
                f = b ^ c ^ d;
                k = 0x6ED9EBA1;
            } else if (j < 60) {
                f = (b & c) | (b & d) | (c & d);
                k = 0x8F1BBCDC;
            } else {
                f = b ^ c ^ d;
                k = 0xCA62C1D6;
            }
            temp = ((a << 5) | (a >> 27)) + f + e + k + w[j];
            e = d;
            d = c;
            c = (b << 30) | (b >> 2);
            b = a;
            a = temp;
        }
        h0 += a; h1 += b; h2 += c; h3 += d; h4 += e;
    }
    shaxun[0] = (h0 >> 24) & 0xFF; shaxun[1] = (h0 >> 16) & 0xFF;
    shaxun[2] = (h0 >> 8) & 0xFF; shaxun[3] = h0 & 0xFF;
    shaxun[4] = (h1 >> 24) & 0xFF; shaxun[5] = (h1 >> 16) & 0xFF;
    shaxun[6] = (h1 >> 8) & 0xFF; shaxun[7] = h1 & 0xFF;
    shaxun[8] = (h2 >> 24) & 0xFF; shaxun[9] = (h2 >> 16) & 0xFF;
    shaxun[10] = (h2 >> 8) & 0xFF; shaxun[11] = h2 & 0xFF;
    shaxun[12] = (h3 >> 24) & 0xFF; shaxun[13] = (h3 >> 16) & 0xFF;
    shaxun[14] = (h3 >> 8) & 0xFF; shaxun[15] = h3 & 0xFF;
    shaxun[16] = (h4 >> 24) & 0xFF; shaxun[17] = (h4 >> 16) & 0xFF;
    shaxun[18] = (h4 >> 8) & 0xFF; shaxun[19] = h4 & 0xFF;
    free(msg);
}

static void shaxun_zifuchuan(const uint8_t shaxun[20], char* out) {
    for (int i = 0; i < 20; i++) {
        sprintf(out + i * 2, "%02x", shaxun[i]);
    }
    out[40] = '\0';
}

static int shaxun_cong_zifuchuan(const char* str, uint8_t shaxun[20]) {
    if (strlen(str) != 40) return 0;
    for (int i = 0; i < 20; i++) {
        char hex[3] = {str[i*2], str[i*2+1], 0};
        shaxun[i] = (uint8_t)strtol(hex, NULL, 16);
    }
    return 1;
}

static int wenjian_cunzai(const char* lujing) {
    return access(lujing, F_OK) == 0;
}

static int wenjian_duqu(const char* lujing, unsigned char** shuju, size_t* changdu) {
    FILE* f = fopen(lujing, "rb");
    if (!f) return 0;
    fseek(f, 0, SEEK_END);
    *changdu = ftell(f);
    fseek(f, 0, SEEK_SET);
    *shuju = malloc(*changdu);
    if (!*shuju) { fclose(f); return 0; }
    size_t read = fread(*shuju, 1, *changdu, f);
    fclose(f);
    return read == *changdu;
}

static int wenjian_xieru(const char* lujing, const unsigned char* shuju, size_t changdu) {
    FILE* f = fopen(lujing, "wb");
    if (!f) return 0;
    size_t written = fwrite(shuju, 1, changdu, f);
    fclose(f);
    return written == changdu;
}

static int wenjian_shanchu(const char* lujing) {
    return remove(lujing) == 0;
}

static int mulu_created(const char* lujing) {
#ifdef _WIN32
    return mkdir(lujing) == 0;
#else
    return mkdir(lujing, 0755) == 0;
#endif
}

static int mulu_cunzai(const char* lujing) {
    struct stat st;
    return stat(lujing, &st) == 0 && S_ISDIR(st.st_mode);
}

static char* lujing_pinjie(const char* jichu, const char* zilujing) {
    static char result[MAX_PATH_LEN];
    int len = strlen(jichu);
    if (len > 0 && jichu[len-1] == PATH_SEP) {
        snprintf(result, sizeof(result), "%s%s", jichu, zilujing);
    } else {
        snprintf(result, sizeof(result), "%s" PATH_SEP_STR "%s", jichu, zilujing);
    }
    return result;
}

static char* lujing_guifanhua(const char* lujing) {
    static char result[MAX_PATH_LEN];
    char temp[MAX_PATH_LEN];
    strcpy(temp, lujing);
    for (char* p = temp; *p; p++) {
        if (*p == '\\') *p = '/';
    }
    char* parts[256];
    int count = 0;
    char* token = strtok(temp, "/");
    while (token && count < 256) {
        if (strcmp(token, ".") == 0) {
            token = strtok(NULL, "/");
            continue;
        }
        if (strcmp(token, "..") == 0) {
            if (count > 0) count--;
            token = strtok(NULL, "/");
            continue;
        }
        parts[count++] = token;
        token = strtok(NULL, "/");
    }
    result[0] = '\0';
    for (int i = 0; i < count; i++) {
        if (i > 0) strcat(result, "/");
        strcat(result, parts[i]);
    }
    if (count == 0) strcpy(result, ".");
    return result;
}

static yuyan_leixing_t shibie_yuyan(const char* wenjian_lujing) {
    const char* dian = strrchr(wenjian_lujing, '.');
    if (!dian) return YUYAN_UNKNOWN;
    char kuozhan[16];
    strncpy(kuozhan, dian, sizeof(kuozhan) - 1);
    kuozhan[sizeof(kuozhan) - 1] = '\0';
    for (char* p = kuozhan; *p; p++) *p = tolower(*p);
    for (int i = 0; kuozhan_biao[i].kuozhan != NULL; i++) {
        if (strcmp(kuozhan, kuozhan_biao[i].kuozhan) == 0) {
            return kuozhan_biao[i].yuyan;
        }
    }
    const char* mingzi = strrchr(wenjian_lujing, PATH_SEP);
    if (!mingzi) mingzi = wenjian_lujing;
    else mingzi++;
    if (strcmp(mingzi, "Makefile") == 0 || strcmp(mingzi, "CMakeLists.txt") == 0) {
        return YUYAN_TEXT;
    }
    return YUYAN_UNKNOWN;
}

static const char* huode_yuyan_mingcheng(yuyan_leixing_t yuyan) {
    for (int i = 0; kuozhan_biao[i].kuozhan != NULL; i++) {
        if (kuozhan_biao[i].yuyan == yuyan) return kuozhan_biao[i].mingcheng;
    }
    return "Unknown";
}

static const char* huode_yuyan_yanse(yuyan_leixing_t yuyan) {
    for (int i = 0; kuozhan_biao[i].kuozhan != NULL; i++) {
        if (kuozhan_biao[i].yuyan == yuyan) return kuozhan_biao[i].yanse;
    }
    return "\033[0m";
}

static int shifou_gvt_cangku(const char* lujing) {
    char gvt_path[MAX_PATH_LEN];
    snprintf(gvt_path, sizeof(gvt_path), "%s" PATH_SEP_STR ".gvt", lujing);
    return mulu_cunzai(gvt_path);
}

static int yasuo_shuju(const unsigned char* shuju, size_t changdu, unsigned char** out, size_t* outlen) {
    uLongf len = compressBound(changdu);
    *out = malloc(len);
    if (!*out) return 0;
    if (compress(*out, &len, shuju, changdu) != Z_OK) {
        free(*out);
        return 0;
    }
    *outlen = len;
    return 1;
}

static int jieya_shuju(const unsigned char* shuju, size_t changdu, unsigned char** out, size_t* outlen) {
    *outlen = changdu * 4;
    *out = malloc(*outlen);
    if (!*out) return 0;
    uLongf len = *outlen;
    int ret = uncompress(*out, &len, shuju, changdu);
    while (ret == Z_BUF_ERROR) {
        free(*out);
        *outlen = *outlen * 4;
        *out = malloc(*outlen);
        if (!*out) return 0;
        len = *outlen;
        ret = uncompress(*out, &len, shuju, changdu);
    }
    if (ret != Z_OK) {
        free(*out);
        return 0;
    }
    *outlen = len;
    return 1;
}

static char* duixiang_lujing(const uint8_t shaxun[20]) {
    static char path[MAX_PATH_LEN];
    char hex[41];
    shaxun_zifuchuan(shaxun, hex);
    snprintf(path, sizeof(path), ".gvt" PATH_SEP_STR "duixiang" PATH_SEP_STR "%c%c" PATH_SEP_STR "%s",
             hex[0], hex[1], hex + 2);
    return path;
}

static int duixiang_cunzai(const uint8_t shaxun[20]) {
    return wenjian_cunzai(duixiang_lujing(shaxun));
}

static int duixiang_xieru(const uint8_t shaxun[20], duixiang_leixing_t leixing, const void* shuju, size_t daxiao) {
    char tou[64];
    const char* leixing_str = "wenjian";
    if (leixing == OBJ_MULU) leixing_str = "mulu";
    else if (leixing == OBJ_TIJIAO) leixing_str = "tijiao";
    else if (leixing == OBJ_BIAOQIAN) leixing_str = "biaoqian";
    int tou_len = snprintf(tou, sizeof(tou), "%s %zu\0", leixing_str, daxiao);
    size_t total = tou_len + daxiao;
    unsigned char* data = malloc(total);
    if (!data) return 0;
    memcpy(data, tou, tou_len);
    memcpy(data + tou_len, shuju, daxiao);
    unsigned char* comp = NULL;
    size_t comp_len = 0;
    if (!yasuo_shuju(data, total, &comp, &comp_len)) {
        free(data);
        return 0;
    }
    free(data);
    char* path = duixiang_lujing(shaxun);
    char dir_path[MAX_PATH_LEN];
    strcpy(dir_path, path);
    char* last_sep = strrchr(dir_path, PATH_SEP);
    if (last_sep) *last_sep = '\0';
    if (!mulu_cunzai(dir_path)) {
        char temp[MAX_PATH_LEN];
        strcpy(temp, dir_path);
        for (char* p = temp; *p; p++) {
            if (*p == PATH_SEP) {
                *p = '\0';
                if (!mulu_cunzai(temp)) mulu_created(temp);
                *p = PATH_SEP;
            }
        }
        if (!mulu_cunzai(temp)) mulu_created(temp);
    }
    int ret = wenjian_xieru(path, comp, comp_len);
    free(comp);
    return ret;
}

static duixiang_t* duixiang_duqu(const uint8_t shaxun[20]) {
    char* path = duixiang_lujing(shaxun);
    if (!wenjian_cunzai(path)) return NULL;
    unsigned char* comp = NULL;
    size_t comp_len = 0;
    if (!wenjian_duqu(path, &comp, &comp_len)) return NULL;
    unsigned char* data = NULL;
    size_t data_len = 0;
    if (!jieya_shuju(comp, comp_len, &data, &data_len)) {
        free(comp);
        return NULL;
    }
    free(comp);
    char leixing_str[32];
    size_t size;
    int parsed = sscanf((char*)data, "%31s %zu", leixing_str, &size);
    if (parsed != 2) {
        free(data);
        return NULL;
    }
    duixiang_t* dx = malloc(sizeof(duixiang_t));
    if (!dx) { free(data); return NULL; }
    memcpy(dx->shaxun, shaxun, 20);
    if (strcmp(leixing_str, "wenjian") == 0) dx->leixing = OBJ_WENJIAN;
    else if (strcmp(leixing_str, "mulu") == 0) dx->leixing = OBJ_MULU;
    else if (strcmp(leixing_str, "tijiao") == 0) dx->leixing = OBJ_TIJIAO;
    else if (strcmp(leixing_str, "biaoqian") == 0) dx->leixing = OBJ_BIAOQIAN;
    else { free(data); free(dx); return NULL; }
    int tou_len = snprintf(NULL, 0, "%s %zu", leixing_str, size);
    dx->shuju = malloc(size);
    if (!dx->shuju) { free(data); free(dx); return NULL; }
    memcpy(dx->shuju, data + tou_len, size);
    dx->daxiao = size;
    free(data);
    return dx;
}

static void duixiang_shifang(duixiang_t* dx) {
    if (dx) {
        if (dx->shuju) free(dx->shuju);
        free(dx);
    }
}

static int paichu_duqu(char paichu[64][256], int* shuliang) {
    if (!wenjian_cunzai("PaiChu.gvt")) { *shuliang = 0; return 1; }
    unsigned char* data = NULL;
    size_t len = 0;
    if (!wenjian_duqu("PaiChu.gvt", &data, &len)) { *shuliang = 0; return 0; }
    int n = 0;
    size_t start = 0;
    for (size_t i = 0; i <= len && n < 64; i++) {
        if (i == len || data[i] == '\n' || data[i] == '\r') {
            size_t l = i - start;
            while (l > 0 && (data[start + l - 1] == ' ' || data[start + l - 1] == '\t')) l--;
            if (l > 0 && l < 256) {
                memcpy(paichu[n], data + start, l);
                paichu[n][l] = '\0';
                n++;
            }
            start = i + 1;
        }
    }
    free(data);
    *shuliang = n;
    return 1;
}

static int wenjian_paichu(const char* mingzi, char paichu[64][256], int shuliang) {
    for (int i = 0; i < shuliang; i++) {
        if (strcmp(mingzi, paichu[i]) == 0) return 1;
    }
    return 0;
}

static int wenjian_cun_chu_duixiang(const char* lujing, uint8_t shaxun[20]) {
    unsigned char* shuju = NULL;
    size_t changdu = 0;
    if (!wenjian_duqu(lujing, &shuju, &changdu)) return 0;
    sha1_jisuan(shuju, changdu, shaxun);
    if (duixiang_cunzai(shaxun)) {
        free(shuju);
        return 1;
    }
    int ret = duixiang_xieru(shaxun, OBJ_WENJIAN, shuju, changdu);
    free(shuju);
    return ret;
}

typedef struct mulu_tiaomu {
    uint16_t mo_shi;
    char ming_zi[256];
    uint8_t shaxun[20];
} mulu_tiaomu_t;

static int mulu_tiaomu_compare(const void* a, const void* b) {
    return strcmp(((mulu_tiaomu_t*)a)->ming_zi, ((mulu_tiaomu_t*)b)->ming_zi);
}

static int mulu_goujian(const char* lujing, const char* xiangdui, uint8_t shaxun[20]) {
    mulu_tiaomu_t* tiaomu = NULL;
    int count = 0;
    char paichu[64][256];
    int paichu_n = 0;
    paichu_duqu(paichu, &paichu_n);
    char quan_lujing[MAX_PATH_LEN];
    snprintf(quan_lujing, sizeof(quan_lujing), "%s" PATH_SEP_STR "%s", lujing, xiangdui);
#ifdef _WIN32
    char pattern[MAX_PATH_LEN];
    snprintf(pattern, sizeof(pattern), "%s" PATH_SEP_STR "*", quan_lujing);
    WIN32_FIND_DATA fd;
    HANDLE h = FindFirstFile(pattern, &fd);
    if (h == INVALID_HANDLE_VALUE) return 0;
    do {
        if (strcmp(fd.cFileName, ".") == 0 || strcmp(fd.cFileName, "..") == 0) continue;
        if (strcmp(fd.cFileName, ".gvt") == 0) continue;
        if (wenjian_paichu(fd.cFileName, paichu, paichu_n)) continue;
        char zilujing[MAX_PATH_LEN];
        snprintf(zilujing, sizeof(zilujing), "%s" PATH_SEP_STR "%s", xiangdui, fd.cFileName);
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            uint8_t sub_sha[20];
            if (!mulu_goujian(lujing, zilujing, sub_sha)) {
                FindClose(h);
                free(tiaomu);
                return 0;
            }
            tiaomu = realloc(tiaomu, (count + 1) * sizeof(mulu_tiaomu_t));
            if (!tiaomu) { FindClose(h); return 0; }
            tiaomu[count].mo_shi = 040000;
            strcpy(tiaomu[count].ming_zi, fd.cFileName);
            memcpy(tiaomu[count].shaxun, sub_sha, 20);
            count++;
        } else {
            uint8_t file_sha[20];
            char file_path[MAX_PATH_LEN];
            snprintf(file_path, sizeof(file_path), "%s" PATH_SEP_STR "%s", quan_lujing, fd.cFileName);
            if (!wenjian_cun_chu_duixiang(file_path, file_sha)) {
                FindClose(h);
                free(tiaomu);
                return 0;
            }
            tiaomu = realloc(tiaomu, (count + 1) * sizeof(mulu_tiaomu_t));
            if (!tiaomu) { FindClose(h); return 0; }
            tiaomu[count].mo_shi = 0100644;
            strcpy(tiaomu[count].ming_zi, fd.cFileName);
            memcpy(tiaomu[count].shaxun, file_sha, 20);
            count++;
        }
    } while (FindNextFile(h, &fd));
    FindClose(h);
#else
    DIR* dir = opendir(quan_lujing);
    if (!dir) return 0;
    struct dirent* entry;
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) continue;
        if (strcmp(entry->d_name, ".gvt") == 0) continue;
        char zilujing[MAX_PATH_LEN];
        snprintf(zilujing, sizeof(zilujing), "%s" PATH_SEP_STR "%s", xiangdui, entry->d_name);
        char full_path[MAX_PATH_LEN];
        snprintf(full_path, sizeof(full_path), "%s" PATH_SEP_STR "%s", quan_lujing, entry->d_name);
        struct stat st;
        if (stat(full_path, &st) != 0) { closedir(dir); free(tiaomu); return 0; }
        if (S_ISDIR(st.st_mode)) {
            uint8_t sub_sha[20];
            if (!mulu_goujian(lujing, zilujing, sub_sha)) {
                closedir(dir);
                free(tiaomu);
                return 0;
            }
            tiaomu = realloc(tiaomu, (count + 1) * sizeof(mulu_tiaomu_t));
            if (!tiaomu) { closedir(dir); return 0; }
            tiaomu[count].mo_shi = 040000;
            strcpy(tiaomu[count].ming_zi, entry->d_name);
            memcpy(tiaomu[count].shaxun, sub_sha, 20);
            count++;
        } else {
            uint8_t file_sha[20];
            if (!wenjian_cun_chu_duixiang(full_path, file_sha)) {
                closedir(dir);
                free(tiaomu);
                return 0;
            }
            tiaomu = realloc(tiaomu, (count + 1) * sizeof(mulu_tiaomu_t));
            if (!tiaomu) { closedir(dir); return 0; }
            tiaomu[count].mo_shi = 0100644;
            strcpy(tiaomu[count].ming_zi, entry->d_name);
            memcpy(tiaomu[count].shaxun, file_sha, 20);
            count++;
        }
    }
    closedir(dir);
#endif
    if (count > 1) {
        qsort(tiaomu, count, sizeof(mulu_tiaomu_t), mulu_tiaomu_compare);
    }
    size_t total = 0;
    for (int i = 0; i < count; i++) {
        total += 6 + strlen(tiaomu[i].ming_zi) + 20;
    }
    unsigned char* data = malloc(total);
    if (!data) { free(tiaomu); return 0; }
    size_t pos = 0;
    for (int i = 0; i < count; i++) {
        memcpy(data + pos, &tiaomu[i].mo_shi, 2);
        pos += 2;
        memcpy(data + pos, &tiaomu[i].shaxun, 20);
        pos += 20;
        uint16_t len = strlen(tiaomu[i].ming_zi);
        memcpy(data + pos, &len, 2);
        pos += 2;
        memcpy(data + pos, tiaomu[i].ming_zi, len);
        pos += len;
    }
    free(tiaomu);
    sha1_jisuan(data, total, shaxun);
    int ret = duixiang_xieru(shaxun, OBJ_MULU, data, total);
    free(data);
    return ret;
}

static suoyin_t* suoyin_duqu() {
    if (!wenjian_cunzai(".gvt/suoyin")) {
        suoyin_t* sy = malloc(sizeof(suoyin_t));
        if (!sy) return NULL;
        sy->tiaomu_shuliang = 0;
        sy->tiaomu = NULL;
        return sy;
    }
    unsigned char* data = NULL;
    size_t len = 0;
    if (!wenjian_duqu(".gvt/suoyin", &data, &len)) return NULL;
    if (len < 4) { free(data); return NULL; }
    uint32_t count = (data[0] << 24) | (data[1] << 16) | (data[2] << 8) | data[3];
    suoyin_t* sy = malloc(sizeof(suoyin_t));
    if (!sy) { free(data); return NULL; }
    sy->tiaomu_shuliang = count;
    sy->tiaomu = malloc(count * sizeof(suoyin_tiaomu_t*));
    if (!sy->tiaomu) { free(data); free(sy); return NULL; }
    size_t pos = 4;
    for (uint32_t i = 0; i < count; i++) {
        if (pos + 72 > len) { free(data); free(sy->tiaomu); free(sy); return NULL; }
        suoyin_tiaomu_t* t = malloc(sizeof(suoyin_tiaomu_t) + 256);
        if (!t) { free(data); free(sy->tiaomu); free(sy); return NULL; }
        t->chuangjian_shijian = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->chuangjian_naimiao = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->xiugai_shijian = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->xiugai_naimiao = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->shebei = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->jiedian = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->quanxian = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->yonghu_id = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->zu_id = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        t->wenjian_daxiao = (data[pos] << 24) | (data[pos+1] << 16) | (data[pos+2] << 8) | data[pos+3]; pos += 4;
        memcpy(t->shaxun, data + pos, 20); pos += 20;
        t->lujing_changdu = (data[pos] << 8) | data[pos+1]; pos += 2;
        if (pos + t->lujing_changdu > len) { free(t); free(data); free(sy->tiaomu); free(sy); return NULL; }
        memcpy(t->lujing, data + pos, t->lujing_changdu);
        t->lujing[t->lujing_changdu] = '\0';
        pos += t->lujing_changdu;
        sy->tiaomu[i] = t;
    }
    free(data);
    return sy;
}

static int suoyin_xieru(suoyin_t* sy) {
    size_t total = 4;
    for (uint32_t i = 0; i < sy->tiaomu_shuliang; i++) {
        total += 72 + 20 + 2 + strlen(sy->tiaomu[i]->lujing);
    }
    unsigned char* data = malloc(total);
    if (!data) return 0;
    size_t pos = 0;
    data[pos++] = (sy->tiaomu_shuliang >> 24) & 0xFF;
    data[pos++] = (sy->tiaomu_shuliang >> 16) & 0xFF;
    data[pos++] = (sy->tiaomu_shuliang >> 8) & 0xFF;
    data[pos++] = sy->tiaomu_shuliang & 0xFF;
    for (uint32_t i = 0; i < sy->tiaomu_shuliang; i++) {
        suoyin_tiaomu_t* t = sy->tiaomu[i];
        data[pos++] = (t->chuangjian_shijian >> 24) & 0xFF;
        data[pos++] = (t->chuangjian_shijian >> 16) & 0xFF;
        data[pos++] = (t->chuangjian_shijian >> 8) & 0xFF;
        data[pos++] = t->chuangjian_shijian & 0xFF;
        data[pos++] = (t->chuangjian_naimiao >> 24) & 0xFF;
        data[pos++] = (t->chuangjian_naimiao >> 16) & 0xFF;
        data[pos++] = (t->chuangjian_naimiao >> 8) & 0xFF;
        data[pos++] = t->chuangjian_naimiao & 0xFF;
        data[pos++] = (t->xiugai_shijian >> 24) & 0xFF;
        data[pos++] = (t->xiugai_shijian >> 16) & 0xFF;
        data[pos++] = (t->xiugai_shijian >> 8) & 0xFF;
        data[pos++] = t->xiugai_shijian & 0xFF;
        data[pos++] = (t->xiugai_naimiao >> 24) & 0xFF;
        data[pos++] = (t->xiugai_naimiao >> 16) & 0xFF;
        data[pos++] = (t->xiugai_naimiao >> 8) & 0xFF;
        data[pos++] = t->xiugai_naimiao & 0xFF;
        data[pos++] = (t->shebei >> 24) & 0xFF;
        data[pos++] = (t->shebei >> 16) & 0xFF;
        data[pos++] = (t->shebei >> 8) & 0xFF;
        data[pos++] = t->shebei & 0xFF;
        data[pos++] = (t->jiedian >> 24) & 0xFF;
        data[pos++] = (t->jiedian >> 16) & 0xFF;
        data[pos++] = (t->jiedian >> 8) & 0xFF;
        data[pos++] = t->jiedian & 0xFF;
        data[pos++] = (t->quanxian >> 24) & 0xFF;
        data[pos++] = (t->quanxian >> 16) & 0xFF;
        data[pos++] = (t->quanxian >> 8) & 0xFF;
        data[pos++] = t->quanxian & 0xFF;
        data[pos++] = (t->yonghu_id >> 24) & 0xFF;
        data[pos++] = (t->yonghu_id >> 16) & 0xFF;
        data[pos++] = (t->yonghu_id >> 8) & 0xFF;
        data[pos++] = t->yonghu_id & 0xFF;
        data[pos++] = (t->zu_id >> 24) & 0xFF;
        data[pos++] = (t->zu_id >> 16) & 0xFF;
        data[pos++] = (t->zu_id >> 8) & 0xFF;
        data[pos++] = t->zu_id & 0xFF;
        data[pos++] = (t->wenjian_daxiao >> 24) & 0xFF;
        data[pos++] = (t->wenjian_daxiao >> 16) & 0xFF;
        data[pos++] = (t->wenjian_daxiao >> 8) & 0xFF;
        data[pos++] = t->wenjian_daxiao & 0xFF;
        memcpy(data + pos, t->shaxun, 20); pos += 20;
        uint16_t len = t->lujing_changdu;
        data[pos++] = (len >> 8) & 0xFF;
        data[pos++] = len & 0xFF;
        memcpy(data + pos, t->lujing, len); pos += len;
    }
    int ret = wenjian_xieru(".gvt/suoyin", data, total);
    free(data);
    return ret;
}

static void suoyin_shifang(suoyin_t* sy) {
    if (sy) {
        for (uint32_t i = 0; i < sy->tiaomu_shuliang; i++) {
            free(sy->tiaomu[i]);
        }
        free(sy->tiaomu);
        free(sy);
    }
}

static int suoyin_tianjia(suoyin_t* sy, const char* lujing, const uint8_t shaxun[20]) {
    for (uint32_t i = 0; i < sy->tiaomu_shuliang; i++) {
        if (strcmp(sy->tiaomu[i]->lujing, lujing) == 0) {
            memcpy(sy->tiaomu[i]->shaxun, shaxun, 20);
            sy->tiaomu[i]->wenjian_daxiao = 0;
            sy->tiaomu[i]->xiugai_shijian = (uint32_t)time(NULL);
            return 1;
        }
    }
    suoyin_tiaomu_t* t = malloc(sizeof(suoyin_tiaomu_t) + strlen(lujing) + 1);
    if (!t) return 0;
    memset(t, 0, sizeof(suoyin_tiaomu_t) + strlen(lujing) + 1);
    t->chuangjian_shijian = (uint32_t)time(NULL);
    t->xiugai_shijian = (uint32_t)time(NULL);
    t->quanxian = 0100644;
    t->wenjian_daxiao = 0;
    memcpy(t->shaxun, shaxun, 20);
    t->lujing_changdu = strlen(lujing);
    strcpy(t->lujing, lujing);
    sy->tiaomu = realloc(sy->tiaomu, (sy->tiaomu_shuliang + 1) * sizeof(suoyin_tiaomu_t*));
    if (!sy->tiaomu) { free(t); return 0; }
    sy->tiaomu[sy->tiaomu_shuliang++] = t;
    return 1;
}

static char* yinyong_duqu(const char* yinyong_ming) {
    static char mubiao[MAX_PATH_LEN];
    char path[MAX_PATH_LEN];
    snprintf(path, sizeof(path), ".gvt/yinyong/%s", yinyong_ming);
    if (!wenjian_cunzai(path)) return NULL;
    unsigned char* data = NULL;
    size_t len = 0;
    if (!wenjian_duqu(path, &data, &len)) return NULL;
    if (len == 0) { free(data); return NULL; }
    size_t n = len;
    if (data[n - 1] == '\n') n--;
    size_t copy = n < MAX_PATH_LEN - 1 ? n : MAX_PATH_LEN - 1;
    memcpy(mubiao, data, copy);
    mubiao[copy] = '\0';
    free(data);
    return mubiao;
}

static int yinyong_xieru(const char* yinyong_ming, const char* mubiao) {
    char path[MAX_PATH_LEN];
    snprintf(path, sizeof(path), ".gvt/yinyong/%s", yinyong_ming);
    char dir_path[MAX_PATH_LEN];
    strcpy(dir_path, path);
    char* last_sep = strrchr(dir_path, PATH_SEP);
    if (!last_sep) last_sep = strrchr(dir_path, '/');
    if (last_sep) *last_sep = '\0';
    if (!mulu_cunzai(dir_path)) {
        char temp[MAX_PATH_LEN];
        strcpy(temp, dir_path);
        for (char* p = temp; *p; p++) {
            if (*p == PATH_SEP) {
                *p = '\0';
                if (!mulu_cunzai(temp)) mulu_created(temp);
                *p = PATH_SEP;
            }
        }
        if (!mulu_cunzai(temp)) mulu_created(temp);
    }
    return wenjian_xieru(path, (unsigned char*)mubiao, strlen(mubiao));
}

static int tou_duqu(char* mubiao, size_t daxiao) {
    if (!wenjian_cunzai(".gvt/TOU")) return 0;
    unsigned char* data = NULL;
    size_t len = 0;
    if (!wenjian_duqu(".gvt/TOU", &data, &len)) return 0;
    if (len == 0) { free(data); return 0; }
    size_t n = len;
    if (data[n - 1] == '\n') n--;
    size_t copy = n < daxiao - 1 ? n : daxiao - 1;
    memcpy(mubiao, data, copy);
    mubiao[copy] = '\0';
    free(data);
    return 1;
}

static int tou_xieru(const char* mubiao) {
    return wenjian_xieru(".gvt/TOU", (unsigned char*)mubiao, strlen(mubiao));
}

static int fencha_dangqian(char* fencha, size_t daxiao) {
    char tou[512];
    if (!tou_duqu(tou, sizeof(tou))) return 0;
    if (strncmp(tou, "yinyong: ", 9) == 0) {
        const char* p = tou + 9;
        if (strncmp(p, "fencha/", 7) == 0) p += 7;
        strncpy(fencha, p, daxiao - 1);
        fencha[daxiao - 1] = '\0';
        return 1;
    }
    return 0;
}

static int shifou_detached() {
    char tou[512];
    if (!tou_duqu(tou, sizeof(tou))) return 1;
    return strncmp(tou, "yinyong: ", 9) != 0;
}

static int tijiao_chuangjian(const char* xiaoxi, const char* zuozhe, const char* youxiang, uint8_t tijiao_hash[20]) {
    suoyin_t* sy = suoyin_duqu();
    if (!sy) return 0;
    uint8_t mulu_sha[20];
    if (!mulu_goujian(".", "", mulu_sha)) {
        suoyin_shifang(sy);
        return 0;
    }
    uint8_t fu_sha[20] = {0};
    char fencha[256];
    if (fencha_dangqian(fencha, sizeof(fencha))) {
        char ref_path[512];
        snprintf(ref_path, sizeof(ref_path), "fencha/%s", fencha);
        char* commit_str = yinyong_duqu(ref_path);
        if (commit_str) {
            shaxun_cong_zifuchuan(commit_str, fu_sha);
        }
    }
    tijiao_t tijiao;
    memset(&tijiao, 0, sizeof(tijiao));
    memcpy(tijiao.mulu_shaxun, mulu_sha, 20);
    memcpy(tijiao.fu_tijiao, fu_sha, 20);
    strncpy(tijiao.zuozhe, zuozhe, sizeof(tijiao.zuozhe) - 1);
    strncpy(tijiao.youxiang, youxiang, sizeof(tijiao.youxiang) - 1);
    tijiao.shijianchuo = time(NULL);
    tijiao.shiqu_pianyi = 0;
    strncpy(tijiao.xiaoxi, xiaoxi, sizeof(tijiao.xiaoxi) - 1);
    sha1_jisuan((unsigned char*)&tijiao, sizeof(tijiao), tijiao_hash);
    if (!duixiang_xieru(tijiao_hash, OBJ_TIJIAO, &tijiao, sizeof(tijiao))) {
        suoyin_shifang(sy);
        return 0;
    }
    if (fencha_dangqian(fencha, sizeof(fencha))) {
        char hex[41];
        shaxun_zifuchuan(tijiao_hash, hex);
        char ref_path[512];
        snprintf(ref_path, sizeof(ref_path), "fencha/%s", fencha);
        yinyong_xieru(ref_path, hex);
    }
    suoyin_shifang(sy);
    return 1;
}

static tijiao_t* tijiao_duqu(const uint8_t shaxun[20]) {
    duixiang_t* dx = duixiang_duqu(shaxun);
    if (!dx || dx->leixing != OBJ_TIJIAO) {
        if (dx) duixiang_shifang(dx);
        return NULL;
    }
    tijiao_t* tijiao = malloc(sizeof(tijiao_t));
    if (!tijiao) { duixiang_shifang(dx); return NULL; }
    memcpy(tijiao, dx->shuju, sizeof(tijiao_t));
    duixiang_shifang(dx);
    return tijiao;
}

static void tijiao_shifang(tijiao_t* tijiao) {
    free(tijiao);
}

static int chushi_mingling(int argc, char** argv) {
    if (shifou_gvt_cangku(".")) {
        printf("错误: 已经是Gvt仓库\n");
        return GVT_ERR_ALREADY_REPO;
    }
    if (!mulu_created(".gvt")) {
        printf("错误: 无法创建 .gvt 目录\n");
        return GVT_ERR_IO;
    }
    if (!mulu_created(".gvt/duixiang")) {
        printf("错误: 无法创建对象目录\n");
        return GVT_ERR_IO;
    }
    if (!mulu_created(".gvt/yinyong")) {
        printf("错误: 无法创建引用目录\n");
        return GVT_ERR_IO;
    }
    if (!mulu_created(".gvt/yinyong/fencha")) {
        printf("错误: 无法创建分支目录\n");
        return GVT_ERR_IO;
    }
    if (!tou_xieru("yinyong: fencha/zhuxian")) {
        printf("错误: 无法写入HEAD\n");
        return GVT_ERR_IO;
    }
    suoyin_t* sy = malloc(sizeof(suoyin_t));
    if (!sy) return GVT_ERR_MEMORY;
    sy->tiaomu_shuliang = 0;
    sy->tiaomu = NULL;
    if (!suoyin_xieru(sy)) {
        suoyin_shifang(sy);
        return GVT_ERR_IO;
    }
    suoyin_shifang(sy);
    printf("初始化空的Gvt仓库成功\n");
    return GVT_OK;
}

static int jia_mulu(suoyin_t* sy, const char* lujing) {
    char paichu[64][256];
    int paichu_n = 0;
    paichu_duqu(paichu, &paichu_n);
    char pattern[MAX_PATH_LEN];
    snprintf(pattern, sizeof(pattern), "%s" PATH_SEP_STR "*", lujing);
#ifdef _WIN32
    WIN32_FIND_DATA fd;
    HANDLE h = FindFirstFile(pattern, &fd);
    if (h == INVALID_HANDLE_VALUE) return 0;
    do {
        if (strcmp(fd.cFileName, ".") == 0 || strcmp(fd.cFileName, "..") == 0) continue;
        if (strcmp(fd.cFileName, ".gvt") == 0) continue;
        if (strcmp(fd.cFileName, ".git") == 0) continue;
        if (wenjian_paichu(fd.cFileName, paichu, paichu_n)) continue;
        char full[MAX_PATH_LEN];
        snprintf(full, sizeof(full), "%s" PATH_SEP_STR "%s", lujing, fd.cFileName);
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            jia_mulu(sy, full);
        } else {
            uint8_t sha[20];
            if (wenjian_cun_chu_duixiang(full, sha)) {
                suoyin_tianjia(sy, full, sha);
            }
        }
    } while (FindNextFile(h, &fd));
    FindClose(h);
#endif
    return 1;
}

static int jia_mingling(int argc, char** argv) {
    if (!shifou_gvt_cangku(".")) {
        printf("错误: 不是Gvt仓库\n");
        return GVT_ERR_NOT_REPO;
    }
    if (argc < 3) {
        printf("用法: gvt jia <wenjian>\n");
        return GVT_ERR_IO;
    }
    suoyin_t* sy = suoyin_duqu();
    if (!sy) return GVT_ERR_IO;
    for (int i = 2; i < argc; i++) {
        char* lujing = lujing_guifanhua(argv[i]);
        if (!wenjian_cunzai(lujing)) {
            printf("警告: %s 不存在，跳过\n", lujing);
            continue;
        }
    struct stat st;
    if (stat(lujing, &st) == 0 && S_ISDIR(st.st_mode)) {
        jia_mulu(sy, lujing);
        continue;
    }
        uint8_t sha[20];
        if (!wenjian_cun_chu_duixiang(lujing, sha)) {
            printf("错误: 无法添加 %s\n", lujing);
            suoyin_shifang(sy);
            return GVT_ERR_IO;
        }
        char xiangdui[MAX_PATH_LEN];
        if (strcmp(lujing, ".") == 0 || strcmp(lujing, "./") == 0) {
            strcpy(xiangdui, "");
        } else {
            strcpy(xiangdui, lujing);
        }
        if (!suoyin_tianjia(sy, xiangdui, sha)) {
            printf("错误: 无法更新索引 %s\n", lujing);
            suoyin_shifang(sy);
            return GVT_ERR_IO;
        }
        yuyan_leixing_t yuyan = shibie_yuyan(lujing);
        printf("添加: %s [%s%s%s]\n", lujing, huode_yuyan_yanse(yuyan), huode_yuyan_mingcheng(yuyan), "\033[0m");
    }
    if (!suoyin_xieru(sy)) {
        suoyin_shifang(sy);
        return GVT_ERR_IO;
    }
    suoyin_shifang(sy);
    return GVT_OK;
}

static int tijiao_mingling(int argc, char** argv) {
    if (!shifou_gvt_cangku(".")) {
        printf("错误: 不是Gvt仓库\n");
        return GVT_ERR_NOT_REPO;
    }
    const char* xiaoxi = "无提交信息";
    const char* zuozhe = getenv("GVT_AUTHOR");
    if (!zuozhe) zuozhe = getenv("USER");
    if (!zuozhe) zuozhe = getenv("USERNAME");
    if (!zuozhe) zuozhe = "Unknown";
    const char* youxiang = getenv("GVT_EMAIL");
    if (!youxiang) youxiang = "unknown@example.com";
    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "-m") == 0 && i + 1 < argc) {
            xiaoxi = argv[i + 1];
            i++;
        } else if (strcmp(argv[i], "--author") == 0 && i + 1 < argc) {
            zuozhe = argv[i + 1];
            i++;
        }
    }
    suoyin_t* sy = suoyin_duqu();
    if (!sy) return GVT_ERR_IO;
    if (sy->tiaomu_shuliang == 0) {
        printf("错误: 没有文件可提交\n");
        suoyin_shifang(sy);
        return GVT_ERR_IO;
    }
    uint8_t tijiao_sha[20];
    if (!tijiao_chuangjian(xiaoxi, zuozhe, youxiang, tijiao_sha)) {
        suoyin_shifang(sy);
        return GVT_ERR_IO;
    }
    suoyin_shifang(sy);
    char hex[41];
    shaxun_zifuchuan(tijiao_sha, hex);
    char fencha[256];
    if (fencha_dangqian(fencha, sizeof(fencha))) {
        printf("[%s %s] %s\n", fencha, hex, xiaoxi);
    } else {
        printf("[%s] %s\n", hex, xiaoxi);
    }
    return GVT_OK;
}

static int rizhi_mingling(int argc, char** argv) {
    if (!shifou_gvt_cangku(".")) {
        printf("错误: 不是Gvt仓库\n");
        return GVT_ERR_NOT_REPO;
    }
    int count = 0;
    int max_count = 100;
    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "-n") == 0 && i + 1 < argc) {
            max_count = atoi(argv[i + 1]);
            i++;
        }
    }
    char commit_hex[41];
    char fencha[256];
    if (fencha_dangqian(fencha, sizeof(fencha))) {
        char ref_path[512];
        snprintf(ref_path, sizeof(ref_path), "fencha/%s", fencha);
        char* commit_str = yinyong_duqu(ref_path);
        if (!commit_str) {
            printf("没有提交记录\n");
            return GVT_OK;
        }
        strcpy(commit_hex, commit_str);
    } else {
        char tou[512];
        if (!tou_duqu(tou, sizeof(tou))) {
            printf("没有提交记录\n");
            return GVT_OK;
        }
        strcpy(commit_hex, tou);
    }
    while (count < max_count) {
        uint8_t sha[20];
        if (!shaxun_cong_zifuchuan(commit_hex, sha)) break;
        tijiao_t* tijiao = tijiao_duqu(sha);
        if (!tijiao) break;
        char tree_hex[41];
        shaxun_zifuchuan(tijiao->mulu_shaxun, tree_hex);
        char parent_hex[41] = "无";
        int has_parent = 0;
        for (int i = 0; i < 20; i++) {
            if (tijiao->fu_tijiao[i] != 0) { has_parent = 1; break; }
        }
        if (has_parent) shaxun_zifuchuan(tijiao->fu_tijiao, parent_hex);
        char time_str[64];
        struct tm* tm_info = localtime(&tijiao->shijianchuo);
        strftime(time_str, sizeof(time_str), "%Y-%m-%d %H:%M:%S", tm_info);
        printf("\033[1;33m提交\033[0m %s\n", commit_hex);
        printf("作者: %s <%s>\n", tijiao->zuozhe, tijiao->youxiang);
        printf("日期: %s\n", time_str);
        printf("    %s\n", tijiao->xiaoxi);
        if (has_parent) {
            strcpy(commit_hex, parent_hex);
            tijiao_shifang(tijiao);
            count++;
            if (count < max_count) printf("\n");
        } else {
            tijiao_shifang(tijiao);
            break;
        }
    }
    return GVT_OK;
}

static int chayi_mingling(int argc, char** argv) {
    if (!shifou_gvt_cangku(".")) {
        printf("错误: 不是Gvt仓库\n");
        return GVT_ERR_NOT_REPO;
    }
    char fencha[256];
    char commit_hex[41];
    if (fencha_dangqian(fencha, sizeof(fencha))) {
        char ref_path[512];
        snprintf(ref_path, sizeof(ref_path), "fencha/%s", fencha);
        char* cs = yinyong_duqu(ref_path);
        if (!cs) {
            printf("没有提交记录\n");
            return GVT_OK;
        }
        strcpy(commit_hex, cs);
    } else {
        char tou[512];
        if (!tou_duqu(tou, sizeof(tou))) {
            printf("没有提交记录\n");
            return GVT_OK;
        }
        strcpy(commit_hex, tou);
    }
    uint8_t sha[20];
    if (!shaxun_cong_zifuchuan(commit_hex, sha)) return GVT_ERR_IO;
    tijiao_t* tj = tijiao_duqu(sha);
    if (!tj) {
        printf("无法读取提交\n");
        return GVT_ERR_IO;
    }
    uint8_t tree_sha[20];
    memcpy(tree_sha, tj->mulu_shaxun, 20);
    tijiao_shifang(tj);
    duixiang_t* dx = duixiang_duqu(tree_sha);
    if (!dx || dx->leixing != OBJ_MULU) {
        if (dx) duixiang_shifang(dx);
        printf("无法读取目录\n");
        return GVT_ERR_IO;
    }
    unsigned char* data = dx->shuju;
    size_t size = dx->daxiao;
    size_t off = 0;
    int changed = 0;
    while (off + 24 <= size) {
        uint16_t ms = (uint16_t)data[off] | ((uint16_t)data[off + 1] << 8);
        uint8_t fsha[20];
        memcpy(fsha, data + off + 2, 20);
        uint16_t nlen = (uint16_t)data[off + 22] | ((uint16_t)data[off + 23] << 8);
        char name[256];
        memcpy(name, data + off + 24, nlen);
        name[nlen] = '\0';
        off += 24 + nlen;
        if (ms != 0100644) continue;
        if (!wenjian_cunzai(name)) {
            printf("删除: %s\n", name);
            changed = 1;
            continue;
        }
        uint8_t wsha[20];
        if (!wenjian_cun_chu_duixiang(name, wsha)) continue;
        if (memcmp(wsha, fsha, 20) != 0) {
            printf("修改: %s\n", name);
            changed = 1;
        }
    }
    duixiang_shifang(dx);
    if (!changed) printf("无差异\n");
    return GVT_OK;
}

static int qiehuan_mingling(int argc, char** argv) {
    if (!shifou_gvt_cangku(".")) {
        printf("错误: 不是Gvt仓库\n");
        return GVT_ERR_NOT_REPO;
    }
    if (argc < 3) {
        printf("用法: gvt qiehuan <fencha|tijiao>\n");
        return GVT_ERR_IO;
    }
    char* mubiao = argv[2];
    uint8_t tijiao_sha[20];
    if (shaxun_cong_zifuchuan(mubiao, tijiao_sha)) {
        if (!duixiang_cunzai(tijiao_sha)) {
            printf("错误: 提交不存在\n");
            return GVT_ERR_INVALID_COMMIT;
        }
        tou_xieru(mubiao);
        printf("切换到提交 %s (detached HEAD)\n", mubiao);
        return GVT_OK;
    }
    char ref_path[512];
    snprintf(ref_path, sizeof(ref_path), "fencha/%s", mubiao);
    char* commit_str = yinyong_duqu(ref_path);
    if (!commit_str) {
        printf("错误: 分支 %s 不存在\n", mubiao);
        return GVT_ERR_INVALID_COMMIT;
    }
    char tou_buf[512];
    snprintf(tou_buf, sizeof(tou_buf), "yinyong: %s", ref_path);
    tou_xieru(tou_buf);
    printf("切换到分支 %s\n", mubiao);
    return GVT_OK;
}

static int fencha_mingling(int argc, char** argv) {
    if (!shifou_gvt_cangku(".")) {
        printf("错误: 不是Gvt仓库\n");
        return GVT_ERR_NOT_REPO;
    }
    if (argc < 3) {
        printf("用法: gvt fencha <mingcheng>\n");
        return GVT_ERR_IO;
    }
    char* mingcheng = argv[2];
    char ref_path[512];
    snprintf(ref_path, sizeof(ref_path), "fencha/%s", mingcheng);
    if (yinyong_duqu(ref_path)) {
        printf("错误: 分支 %s 已存在\n", mingcheng);
        return GVT_ERR_BRANCH_EXISTS;
    }
    char commit_hex[41];
    char fencha[256];
    if (fencha_dangqian(fencha, sizeof(fencha))) {
        char current_ref[512];
        snprintf(current_ref, sizeof(current_ref), "fencha/%s", fencha);
        char* commit_str = yinyong_duqu(current_ref);
        if (commit_str) strcpy(commit_hex, commit_str);
        else return GVT_ERR_INVALID_COMMIT;
    } else {
        char tou[512];
        if (!tou_duqu(tou, sizeof(tou))) return GVT_ERR_INVALID_COMMIT;
        strcpy(commit_hex, tou);
    }
    if (!yinyong_xieru(ref_path, commit_hex)) {
        printf("错误: 无法创建分支\n");
        return GVT_ERR_IO;
    }
    printf("创建分支 %s 成功\n", mingcheng);
    return GVT_OK;
}

static void yonghu_bangzhu() {
    printf("Gvt 版本控制系统 v%s\n", GVT_VERSION);
    printf("\n用法: gvt <mingling> [canshu]\n");
    printf("\n命令:\n");
    printf("  chushi              初始化仓库\n");
    printf("  jia <wenjian...>    添加文件到索引\n");
    printf("  tijiao -m <xiaoxi>  提交变更\n");
    printf("  rizhi [-n <shuliang>] 查看提交历史\n");
    printf("  chayi               显示差异\n");
    printf("  qiehuan <fencha|tijiao> 切换分支或提交\n");
    printf("  fencha <mingcheng>  创建新分支\n");
    printf("  bangzhu             显示此帮助\n");
}

int main(int argc, char** argv) {
    if (argc < 2) {
        yonghu_bangzhu();
        return 0;
    }
    int ret = GVT_OK;
    if (strcmp(argv[1], "chushi") == 0) {
        ret = chushi_mingling(argc, argv);
    } else if (strcmp(argv[1], "jia") == 0) {
        ret = jia_mingling(argc, argv);
    } else if (strcmp(argv[1], "tijiao") == 0) {
        ret = tijiao_mingling(argc, argv);
    } else if (strcmp(argv[1], "rizhi") == 0) {
        ret = rizhi_mingling(argc, argv);
    } else if (strcmp(argv[1], "chayi") == 0) {
        ret = chayi_mingling(argc, argv);
    } else if (strcmp(argv[1], "qiehuan") == 0) {
        ret = qiehuan_mingling(argc, argv);
    } else if (strcmp(argv[1], "fencha") == 0) {
        ret = fencha_mingling(argc, argv);
    } else if (strcmp(argv[1], "bangzhu") == 0 || strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0) {
        yonghu_bangzhu();
    } else {
        printf("未知命令: %s\n", argv[1]);
        printf("使用 'gvt bangzhu' 查看帮助\n");
        ret = GVT_ERR_IO;
    }
    if (ret != GVT_OK && ret != 0) {
        const char* msg = gvt_cuowu_zifuchuan(ret);
        if (msg) printf("错误: %s\n", msg);
    }
    return ret == GVT_OK ? 0 : 1;
}
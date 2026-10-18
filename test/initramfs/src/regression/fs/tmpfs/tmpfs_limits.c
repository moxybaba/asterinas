// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/vfs.h>
#include <unistd.h>

#include "../../common/test.h"

#define MNT "/tmp/tmpfs_limits_mnt"
#define PAGE 4096
#define MIB ((size_t)1024 * 1024)

static char buf[2 * PAGE];

FN_SETUP(mount_point)
{
    CHECK_WITH(mkdir(MNT, 0755), _ret == 0 || errno == EEXIST);
}
END_SETUP()

static size_t fill(int fd, size_t limit, int *err)
{
    size_t total = 0;

    *err = 0;
    while (total < limit) {
        ssize_t n = write(fd, buf, PAGE);
        if (n < 0) {
            *err = errno;
            break;
        }
        total += n;
    }
    errno = 0;
    return total;
}

FN_TEST(size_limit_enforced)
{
    int err;

    TEST_SUCC(mount("tmpfs", MNT, "tmpfs", 0, "size=1m"));
    int fd = TEST_SUCC(open(MNT "/f", O_CREAT | O_RDWR, 0644));
    TEST_RES(fill(fd, 2 * MIB, &err), _ret == MIB && err == ENOSPC);
    TEST_SUCC(close(fd));
    TEST_SUCC(umount(MNT));
}
END_TEST()

FN_TEST(statfs_tracks_usage)
{
    struct statfs st;

    TEST_SUCC(mount("tmpfs", MNT, "tmpfs", 0, "size=1m"));
    TEST_RES(statfs(MNT, &st), (size_t)st.f_blocks * st.f_bsize == MIB);
    int fd = TEST_SUCC(open(MNT "/f", O_CREAT | O_RDWR, 0644));
    TEST_RES(write(fd, buf, 2 * PAGE), _ret == 2 * PAGE);
    TEST_RES(statfs(MNT, &st),
             (size_t)(st.f_blocks - st.f_bfree) * st.f_bsize == 2 * PAGE);
    TEST_SUCC(close(fd));
    TEST_SUCC(umount(MNT));
}
END_TEST()

FN_TEST(nr_inodes_enforced)
{
    char path[64];
    int created = 0, err = 0;

    TEST_SUCC(mount("tmpfs", MNT, "tmpfs", 0, "nr_inodes=10"));
    for (int i = 0; i < 20; i++) {
        snprintf(path, sizeof(path), MNT "/f%d", i);
        int fd = open(path, O_CREAT | O_RDWR, 0644);
        if (fd < 0) {
            err = errno;
            break;
        }
        close(fd);
        created++;
    }
    errno = 0;
    TEST_RES(created, created == 9 && err == ENOSPC);
    TEST_SUCC(umount(MNT));
}
END_TEST()

FN_TEST(unknown_option_rejected)
{
    int ret = TEST_ERRNO(mount("tmpfs", MNT, "tmpfs", 0, "bogus=1"), EINVAL);

    if (ret == 0) {
        umount(MNT);
    }
}
END_TEST()

FN_TEST(root_mode_default)
{
    struct stat st;

    TEST_SUCC(mount("tmpfs", MNT, "tmpfs", 0, NULL));
    TEST_RES(stat(MNT, &st), (st.st_mode & 07777) == 01777);
    TEST_SUCC(umount(MNT));
}
END_TEST()

FN_TEST(sparse_file_has_no_blocks)
{
    struct stat st;

    TEST_SUCC(mount("tmpfs", MNT, "tmpfs", 0, NULL));
    int fd = TEST_SUCC(open(MNT "/s", O_CREAT | O_RDWR, 0644));
    TEST_SUCC(ftruncate(fd, (off_t)1 << 30));
    TEST_RES(fstat(fd, &st), st.st_blocks == 0);
    TEST_SUCC(close(fd));
    TEST_SUCC(umount(MNT));
}
END_TEST()

FN_TEST(remount_resizes)
{
    struct statfs st;

    TEST_SUCC(mount("tmpfs", MNT, "tmpfs", 0, "size=1m"));
    TEST_SUCC(mount("tmpfs", MNT, "tmpfs", MS_REMOUNT, "size=2m"));
    TEST_RES(statfs(MNT, &st), (size_t)st.f_blocks * st.f_bsize == 2 * MIB);
    TEST_SUCC(umount(MNT));
}
END_TEST()

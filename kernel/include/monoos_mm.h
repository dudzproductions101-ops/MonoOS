/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _MONOOS_MM_H
#define _MONOOS_MM_H

#include <linux/mm.h>
#include <linux/gfp.h>

struct page *monoos_alloc_pages(unsigned int order, gfp_t extra_flags);
void         monoos_free_pages(struct page *page, unsigned int order);
void        *monoos_vmalloc(size_t size, gfp_t gfp);

#endif /* _MONOOS_MM_H */

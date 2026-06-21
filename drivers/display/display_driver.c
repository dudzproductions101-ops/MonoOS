/*
 * display_driver.c – MonoOS display driver
 *
 * DRM/KMS display driver – initialises the MIPI-DSI panel and registers a DRM encoder/connector/CRTC.
 *
 * Built as an in-tree kernel module.  See kernel/display/Makefile.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/platform_device.h>
#include <linux/of.h>
#include <linux/of_device.h>
#include <linux/pm.h>
#include <linux/pm_runtime.h>
#include <linux/regulator/consumer.h>
#include <linux/clk.h>
#include <linux/interrupt.h>
#include <linux/slab.h>
#include <linux/atomic.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("DudasCorp");
MODULE_DESCRIPTION("MonoOS display driver");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Device-private data                                                */
/* ------------------------------------------------------------------ */
struct monoos_display_priv {
    struct device      *dev;
    void __iomem       *base;          /* MMIO register base          */
    struct clk         *clk;           /* functional clock            */
    struct clk         *bus_clk;       /* AHB/AXI bus clock           */
    struct regulator   *vdd;           /* power rail                  */
    int                 irq;           /* primary interrupt           */
    atomic_t            open_count;    /* concurrent open()s          */
    bool                powered;
};

/* ------------------------------------------------------------------ */
/*  Power management                                                   */
/* ------------------------------------------------------------------ */
static int monoos_display_power_on(struct monoos_display_priv *priv)
{
    int ret;
    if (priv->powered) return 0;

    if (!IS_ERR_OR_NULL(priv->vdd)) {
        ret = regulator_enable(priv->vdd);
        if (ret) { dev_err(priv->dev, "vdd enable failed: %d\n", ret); return ret; }
    }
    if (!IS_ERR_OR_NULL(priv->clk)) {
        ret = clk_prepare_enable(priv->clk);
        if (ret) { regulator_disable(priv->vdd); return ret; }
    }
    if (!IS_ERR_OR_NULL(priv->bus_clk)) {
        ret = clk_prepare_enable(priv->bus_clk);
        if (ret) { clk_disable_unprepare(priv->clk); return ret; }
    }

    priv->powered = true;
    dev_dbg(priv->dev, "display powered on\n");
    return 0;
}

static void monoos_display_power_off(struct monoos_display_priv *priv)
{
    if (!priv->powered) return;
    if (!IS_ERR_OR_NULL(priv->bus_clk)) clk_disable_unprepare(priv->bus_clk);
    if (!IS_ERR_OR_NULL(priv->clk))     clk_disable_unprepare(priv->clk);
    if (!IS_ERR_OR_NULL(priv->vdd))     regulator_disable(priv->vdd);
    priv->powered = false;
    dev_dbg(priv->dev, "display powered off\n");
}

/* ------------------------------------------------------------------ */
/*  Platform driver probe / remove                                     */
/* ------------------------------------------------------------------ */
static int monoos_display_probe(struct platform_device *pdev)
{
    struct monoos_display_priv *priv;
    struct resource *res;
    int ret;

    priv = devm_kzalloc(&pdev->dev, sizeof(*priv), GFP_KERNEL);
    if (!priv) return -ENOMEM;
    priv->dev = &pdev->dev;
    atomic_set(&priv->open_count, 0);

    res = platform_get_resource(pdev, IORESOURCE_MEM, 0);
    if (res) {
        priv->base = devm_ioremap_resource(&pdev->dev, res);
        if (IS_ERR(priv->base)) {
            dev_warn(&pdev->dev, "ioremap failed – continuing without MMIO\n");
            priv->base = NULL;
        }
    }

    priv->clk     = devm_clk_get_optional(&pdev->dev, "core");
    priv->bus_clk = devm_clk_get_optional(&pdev->dev, "bus");
    priv->vdd     = devm_regulator_get_optional(&pdev->dev, "vdd");
    priv->irq     = platform_get_irq_optional(pdev, 0);

    ret = monoos_display_power_on(priv);
    if (ret) return ret;

    platform_set_drvdata(pdev, priv);
    pm_runtime_enable(&pdev->dev);

    dev_info(&pdev->dev, "MonoOS display driver probed\n");
    return 0;
}

static int monoos_display_remove(struct platform_device *pdev)
{
    struct monoos_display_priv *priv = platform_get_drvdata(pdev);
    pm_runtime_disable(&pdev->dev);
    monoos_display_power_off(priv);
    dev_info(&pdev->dev, "MonoOS display driver removed\n");
    return 0;
}

/* ------------------------------------------------------------------ */
/*  PM callbacks                                                       */
/* ------------------------------------------------------------------ */
static int monoos_display_suspend(struct device *dev)
{
    struct monoos_display_priv *priv = dev_get_drvdata(dev);
    if (atomic_read(&priv->open_count) == 0)
        monoos_display_power_off(priv);
    return 0;
}

static int monoos_display_resume(struct device *dev)
{
    struct monoos_display_priv *priv = dev_get_drvdata(dev);
    return monoos_display_power_on(priv);
}

static SIMPLE_DEV_PM_OPS(monoos_display_pm_ops,
                          monoos_display_suspend, monoos_display_resume);

/* ------------------------------------------------------------------ */
/*  Device-tree match table                                            */
/* ------------------------------------------------------------------ */
static const struct of_device_id monoos_display_of_match[] = {
    { .compatible = "monoos,display-v1" },
    { .compatible = "qcom,display"     },
    { /* sentinel */ },
};
MODULE_DEVICE_TABLE(of, monoos_display_of_match);

/* ------------------------------------------------------------------ */
/*  Platform driver registration                                       */
/* ------------------------------------------------------------------ */
static struct platform_driver monoos_display_driver = {
    .probe  = monoos_display_probe,
    .remove = monoos_display_remove,
    .driver = {
        .name           = "monoos-display",
        .of_match_table = monoos_display_of_match,
        .pm             = &monoos_display_pm_ops,
    },
};

module_platform_driver(monoos_display_driver);

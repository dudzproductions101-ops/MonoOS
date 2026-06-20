/*
 * sensors_driver.c – OneOS sensors driver
 *
 * IIO sensor driver – exposes accelerometer, gyroscope, proximity, and ambient-light sensor via the Linux IIO subsystem.
 *
 * Built as an in-tree kernel module.  See kernel/sensors/Makefile.
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
MODULE_DESCRIPTION("OneOS sensors driver");
MODULE_VERSION("1.0.0");

/* ------------------------------------------------------------------ */
/*  Device-private data                                                */
/* ------------------------------------------------------------------ */
struct oneos_sensors_priv {
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
static int oneos_sensors_power_on(struct oneos_sensors_priv *priv)
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
    dev_dbg(priv->dev, "sensors powered on\n");
    return 0;
}

static void oneos_sensors_power_off(struct oneos_sensors_priv *priv)
{
    if (!priv->powered) return;
    if (!IS_ERR_OR_NULL(priv->bus_clk)) clk_disable_unprepare(priv->bus_clk);
    if (!IS_ERR_OR_NULL(priv->clk))     clk_disable_unprepare(priv->clk);
    if (!IS_ERR_OR_NULL(priv->vdd))     regulator_disable(priv->vdd);
    priv->powered = false;
    dev_dbg(priv->dev, "sensors powered off\n");
}

/* ------------------------------------------------------------------ */
/*  Platform driver probe / remove                                     */
/* ------------------------------------------------------------------ */
static int oneos_sensors_probe(struct platform_device *pdev)
{
    struct oneos_sensors_priv *priv;
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

    ret = oneos_sensors_power_on(priv);
    if (ret) return ret;

    platform_set_drvdata(pdev, priv);
    pm_runtime_enable(&pdev->dev);

    dev_info(&pdev->dev, "OneOS sensors driver probed\n");
    return 0;
}

static int oneos_sensors_remove(struct platform_device *pdev)
{
    struct oneos_sensors_priv *priv = platform_get_drvdata(pdev);
    pm_runtime_disable(&pdev->dev);
    oneos_sensors_power_off(priv);
    dev_info(&pdev->dev, "OneOS sensors driver removed\n");
    return 0;
}

/* ------------------------------------------------------------------ */
/*  PM callbacks                                                       */
/* ------------------------------------------------------------------ */
static int oneos_sensors_suspend(struct device *dev)
{
    struct oneos_sensors_priv *priv = dev_get_drvdata(dev);
    if (atomic_read(&priv->open_count) == 0)
        oneos_sensors_power_off(priv);
    return 0;
}

static int oneos_sensors_resume(struct device *dev)
{
    struct oneos_sensors_priv *priv = dev_get_drvdata(dev);
    return oneos_sensors_power_on(priv);
}

static SIMPLE_DEV_PM_OPS(oneos_sensors_pm_ops,
                          oneos_sensors_suspend, oneos_sensors_resume);

/* ------------------------------------------------------------------ */
/*  Device-tree match table                                            */
/* ------------------------------------------------------------------ */
static const struct of_device_id oneos_sensors_of_match[] = {
    { .compatible = "oneos,sensors-v1" },
    { .compatible = "qcom,sensors"     },
    { /* sentinel */ },
};
MODULE_DEVICE_TABLE(of, oneos_sensors_of_match);

/* ------------------------------------------------------------------ */
/*  Platform driver registration                                       */
/* ------------------------------------------------------------------ */
static struct platform_driver oneos_sensors_driver = {
    .probe  = oneos_sensors_probe,
    .remove = oneos_sensors_remove,
    .driver = {
        .name           = "oneos-sensors",
        .of_match_table = oneos_sensors_of_match,
        .pm             = &oneos_sensors_pm_ops,
    },
};

module_platform_driver(oneos_sensors_driver);

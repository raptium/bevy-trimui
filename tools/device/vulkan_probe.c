/* Enumerate Vulkan capabilities without creating a surface or drawing.
 * Dynamically loads the installed Vulkan library; no loader is bundled.
 */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <vulkan/vulkan.h>

static void version(const char *label, uint32_t v) {
    printf("%s=%u.%u.%u (0x%08x)\n", label, VK_VERSION_MAJOR(v),
           VK_VERSION_MINOR(v), VK_VERSION_PATCH(v), v);
}

int main(void) {
    void *lib = dlopen("libvulkan.so.1", RTLD_NOW | RTLD_LOCAL);
    if (!lib) lib = dlopen("libvulkan.so", RTLD_NOW | RTLD_LOCAL);
    if (!lib) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }
    PFN_vkGetInstanceProcAddr get = (PFN_vkGetInstanceProcAddr)dlsym(lib, "vkGetInstanceProcAddr");
    if (!get) { fprintf(stderr, "missing vkGetInstanceProcAddr\n"); return 2; }
    typedef VkResult (VKAPI_PTR *EnumVersion)(uint32_t *);
    EnumVersion enum_version = (EnumVersion)get(VK_NULL_HANDLE, "vkEnumerateInstanceVersion");
    uint32_t api = VK_API_VERSION_1_0;
    if (enum_version) {
        VkResult result = enum_version(&api);
        printf("vkEnumerateInstanceVersion=%d\n", result);
    }
    version("loader_api", api);
    PFN_vkEnumerateInstanceExtensionProperties enum_ext =
        (PFN_vkEnumerateInstanceExtensionProperties)get(VK_NULL_HANDLE, "vkEnumerateInstanceExtensionProperties");
    uint32_t count = 0;
    VkResult result = enum_ext(NULL, &count, NULL);
    printf("instance_extension_query=%d count=%u\n", result, count);
    VkExtensionProperties *exts = calloc(count ? count : 1, sizeof(*exts));
    if (!exts) return 3;
    result = enum_ext(NULL, &count, exts);
    if (result != VK_SUCCESS && result != VK_INCOMPLETE) return 4;
    for (uint32_t i = 0; i < count; ++i) printf("instance_extension=%s version=%u\n", exts[i].extensionName, exts[i].specVersion);
    free(exts);
    VkApplicationInfo app = { .sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
        .pApplicationName = "brick-capability-probe", .apiVersion = VK_API_VERSION_1_0 };
    VkInstanceCreateInfo info = { .sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO, .pApplicationInfo = &app };
    PFN_vkCreateInstance create = (PFN_vkCreateInstance)get(VK_NULL_HANDLE, "vkCreateInstance");
    VkInstance instance;
    result = create(&info, NULL, &instance);
    printf("vkCreateInstance=%d\n", result);
    if (result != VK_SUCCESS) return 5;
    PFN_vkEnumeratePhysicalDevices enum_devices = (PFN_vkEnumeratePhysicalDevices)get(instance, "vkEnumeratePhysicalDevices");
    PFN_vkGetPhysicalDeviceProperties properties = (PFN_vkGetPhysicalDeviceProperties)get(instance, "vkGetPhysicalDeviceProperties");
    PFN_vkGetPhysicalDeviceFeatures features = (PFN_vkGetPhysicalDeviceFeatures)get(instance, "vkGetPhysicalDeviceFeatures");
    PFN_vkGetPhysicalDeviceQueueFamilyProperties queues = (PFN_vkGetPhysicalDeviceQueueFamilyProperties)get(instance, "vkGetPhysicalDeviceQueueFamilyProperties");
    PFN_vkEnumerateDeviceExtensionProperties device_ext = (PFN_vkEnumerateDeviceExtensionProperties)get(instance, "vkEnumerateDeviceExtensionProperties");
    count = 0;
    result = enum_devices(instance, &count, NULL);
    printf("vkEnumeratePhysicalDevices=%d count=%u\n", result, count);
    VkPhysicalDevice *devices = calloc(count ? count : 1, sizeof(*devices));
    if (!devices) return 6;
    result = enum_devices(instance, &count, devices);
    if (result != VK_SUCCESS && result != VK_INCOMPLETE) return 7;
    for (uint32_t i = 0; i < count; ++i) {
        VkPhysicalDeviceProperties p;
        VkPhysicalDeviceFeatures f;
        properties(devices[i], &p); features(devices[i], &f);
        printf("device=%u name=%s vendor=0x%x device_id=0x%x type=%u\n", i, p.deviceName, p.vendorID, p.deviceID, p.deviceType);
        version("device_api", p.apiVersion);
        /* Vulkan driverVersion encoding is vendor-specific. */
        printf("driver_version_raw=0x%08x\n", p.driverVersion);
        printf("maxImageDimension2D=%u maxBoundDescriptorSets=%u maxPerStageDescriptorStorageBuffers=%u maxStorageBufferRange=%u\n",
            p.limits.maxImageDimension2D, p.limits.maxBoundDescriptorSets,
            p.limits.maxPerStageDescriptorStorageBuffers, p.limits.maxStorageBufferRange);
        printf("features samplerAnisotropy=%u independentBlend=%u fragmentStoresAndAtomics=%u shaderInt16=%u shaderInt64=%u textureCompressionETC2=%u textureCompressionASTC_LDR=%u\n",
            f.samplerAnisotropy, f.independentBlend, f.fragmentStoresAndAtomics,
            f.shaderInt16, f.shaderInt64, f.textureCompressionETC2, f.textureCompressionASTC_LDR);
        uint32_t n = 0; queues(devices[i], &n, NULL);
        VkQueueFamilyProperties *q = calloc(n ? n : 1, sizeof(*q));
        if (!q) return 8;
        queues(devices[i], &n, q);
        for (uint32_t j = 0; j < n; ++j) printf("queue=%u flags=0x%x count=%u\n", j, q[j].queueFlags, q[j].queueCount);
        free(q);
        n = 0; result = device_ext(devices[i], NULL, &n, NULL);
        printf("device_extension_query=%d count=%u\n", result, n);
        exts = calloc(n ? n : 1, sizeof(*exts));
        if (!exts) return 9;
        result = device_ext(devices[i], NULL, &n, exts);
        if (result != VK_SUCCESS && result != VK_INCOMPLETE) return 10;
        for (uint32_t j = 0; j < n; ++j) printf("device_extension=%s version=%u\n", exts[j].extensionName, exts[j].specVersion);
        free(exts);
    }
    free(devices);
    ((PFN_vkDestroyInstance)get(instance, "vkDestroyInstance"))(instance, NULL);
    dlclose(lib);
    return count ? 0 : 11;
}

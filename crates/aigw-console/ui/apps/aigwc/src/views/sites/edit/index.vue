<script lang="ts" setup>
import { computed, onMounted, markRaw, h, ref } from 'vue';
import { useRoute } from 'vue-router';
import { $t } from '#/locales';

import { Page, Loading } from '@vben/common-ui';
import { useTabs } from '@vben/hooks';
import { getSiteApi, updateSiteApi, getAllClustersApi } from '#/api';
import { useVbenForm, z } from '#/adapter/form';
import { message, Card } from 'ant-design-vue';


import DynamicLocation from '#/components/DynamicLocation.vue';
const RawDynamicLocation = markRaw(DynamicLocation);

const route = useRoute();

const { setTabTitle } = useTabs();

const index = computed(() => {
  return route.params?.id ?? -1;
});

setTabTitle(`${index.value} - 详情信息`);

const submitting = ref(true);

const defaultValue = ref([]);

const [Form, formApi] = useVbenForm({
  handleSubmit: onSubmit,
  schema: [
    {
      component: 'ApiSelect',
      componentProps: {
        afterFetch: (data: { name: string; }[]) => {
          return data.map((item: any) => ({
            label: item.name,
            value: item.name,
          }));
        },
        api: getAllClustersApi,
      },
      fieldName: 'cluster',
      label: 'Cluster',
    },
    {
      component: 'Input',
      componentProps: {
        placeholder: 'test.com',
        readonly: true,
        autocomplete: 'off',
      },
      fieldName: 'name',
      label: 'Name',
      rules: z.string().min(3, { message: 'Enter at least 3 letters' }),
    },
    {
      component: 'Input',
      componentProps: {
        placeholder: 'www.test.com,abc.test.com',
      },
      defaultValue: '',
      fieldName: 'alt_names',
      label: 'Alternative Names',
    },
    {
      component: 'Input',
      componentProps: {
        placeholder: '/opt/aigw/data/www/',
      },
      fieldName: 'root_dir',
      label: 'Root Directory',
      rules: z.string().default('').optional(),
    },
    {
      component: 'Switch',
      defaultValue: false,
      fieldName: 'auto_index',
      label: 'Auto Index',
    },
    {
      component: 'Switch',
      defaultValue: false,
      fieldName: 'tls_on',
      label: 'Enable TLS',
    },
    {
      component: 'Switch',
      defaultValue: false,
      fieldName: 'acme_on',
      dependencies: {
        if(values) {
          return !!values.tls_on;
        },
        triggerFields: ['tls_on'],
      },
      label: 'Enable with Let\'s Encrypt',
    },
    {
      component: 'Textarea',
      dependencies: {
        if(values) {
          return !!values.tls_on && !!!values.acme_on;
        },
        triggerFields: ['tls_on', 'acme_on'],
      },
      fieldName: 'tls_cert',
      label: 'Certificate',
      componentProps: {
        placeholder: '-----BEGIN CERTIFICATE-----',
        rows: 20,
        class: 'font-mono',
      },
      rules: 'required',
    },
    {
      component: 'Textarea',
      dependencies: {
        if(values) {
          return !!values.tls_on && !!!values.acme_on;
        },
        triggerFields: ['tls_on', 'acme_on'],
      },
      fieldName: 'tls_private_key',
      label: 'Private Key',
      componentProps: {
        placeholder: '-----BEGIN PRIVATE KEY-----',
        rows: 20,
        class: 'font-mono',
      },
      rules: 'required',
    },
    {
      component: 'Divider',
      fieldName: '_divider',
      formItemClass: '',
      hideLabel: true,
      renderComponentContent: () => {
        return {
          default: () => h('div', 'Locations'),
        };
      },
    },
    {
      component: RawDynamicLocation,
      fieldName: 'locations',
      hideLabel: true,
      formItemClass: '',
      defaultValue: defaultValue,
      componentProps: {
        min: 1,
        max: 10,
        namePath: ['locations'],
      },
    },
  ],
  wrapperClass: 'grid-cols-1',
  commonConfig: {
    labelWidth: 200
  },
});


const fetchData = async () => {
  const site = await getSiteApi(`${index.value}`);

  site.alt_names = site.alt_names.toString()
  site.tls_cert = site.tls_cert ? atob(site.tls_cert) : ""
  site.tls_private_key = site.tls_private_key ? atob(site.tls_private_key) : ""
  site.locations = site.locations.map((location: any) => ({
    ...location,
    upstream: location.upstream.toString().replaceAll(',', '\n'),
    proxy_add_headers: location.proxy_add_headers.length == 0 ? [{
      name: "",
      value: "",
    }] : location.proxy_add_headers,
    proxy_set_headers: location.proxy_set_headers.length == 0 ? [{
      name: "",
      value: "",
    }] : location.proxy_set_headers,
  }))

  defaultValue.value = site.locations;
  formApi.setValues(site);
  submitting.value = false;
}

async function handleAsyncSubmit(values: Record<string, any>) {

  try {
    const locations = values.locations;

    for (let i = 0; i < locations.length; i++) {
      const item = locations[i];
      if (!item.path?.trim()) {
        message.error(`Location ${i + 1}: Path is required`);
        return;
      }
      if (item.proxy && !item.upstream?.trim()) {
        message.error(`Location ${i + 1}: Upstream is required`);
        return;
      } else if (!item.proxy && !item.root_dir?.trim()) {
        message.error(`Location ${i + 1}: Root Directory is required`);
        return;
      }
    }

    const processedValues = {
      ...values,
      tls_cert: values.tls_cert ? btoa(values.tls_cert) : "",
      tls_private_key: values.tls_private_key ? btoa(values.tls_private_key) : "",
      alt_names: !!values.alt_names ? values.alt_names.split(',').map((host: string) => host.trim()).filter(Boolean) : [],
      locations: values.locations ? values.locations.map((location: any) => ({
        ...location,
        upstream: location.upstream ? location.upstream.split('\n').map((u: string) => u.trim()).filter(Boolean) : [],
        lb: location.upstream ? location.upstream.split('\n').map((u: string) => u.trim()).filter(Boolean) : []
      })) : []
    };

    submitting.value = true;
    await updateSiteApi(values.name, processedValues);
    message.success({
      content: `Upadte site successfully!`,
    });
  } catch {

  } finally {
    submitting.value = false;
  }
}
function onSubmit(values: Record<string, any>) {
  handleAsyncSubmit(values).catch(error => {
    console.error('Submit error:', error);
  });

}


onMounted(() => {
  fetchData()
})

</script>

<template>
  <Page content-class="flex flex-col gap-4" description="" title="">
    <Card :title="$t('page.site.edit')">
      <Form />

      <div v-if="submitting" class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-30">
        <Loading :spinning="submitting" />
      </div>
    </Card>
  </Page>
</template>
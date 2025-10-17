<script lang="ts" setup>
import { message, Card } from 'ant-design-vue';
import { h, ref, markRaw } from 'vue';
import { Page, Loading } from '@vben/common-ui';
import { $t } from '#/locales';
import { addSiteApi, getAllClustersApi } from '#/api';

import DynamicLocation from '#/components/DynamicLocation.vue';
const RawDynamicLocation = markRaw(DynamicLocation);

import { useVbenForm, z } from '#/adapter/form';

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
        autoSelect: 'first',
      },
      fieldName: 'cluster',
      label: 'Cluster',
    },
    {
      component: 'Input',
      componentProps: {
        placeholder: 'test.com',
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
      defaultValue: [{
        path: '',
        proxy: false,
        protocol: 'http',
        connection_timeout: 5,
        read_timeout: 5,
        write_timeout: 5,
        idle_timeout: 30,
        sni: "",
        client_max_body_size: 0,
        rewrite: "",
        upstream: "",
        root_dir: "",
        auto_index: false,
      }],
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

const submitting = ref(false);


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
    await addSiteApi(processedValues);
    formApi.resetForm();
    message.success({
      content: `Add site successfully!`,
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
</script>

<template>
  <Page content-class="flex flex-col gap-4" description="" title="">
    <Card :title="$t('page.site.new')">
      <Form />

      <div v-if="submitting" class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-30">
        <Loading :spinning="submitting" />
      </div>
    </Card>
  </Page>
</template>
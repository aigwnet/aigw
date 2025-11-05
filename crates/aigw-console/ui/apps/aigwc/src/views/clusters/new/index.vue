<script lang="ts" setup>
import { message, Card } from 'ant-design-vue';
import { ref } from 'vue';
import { Page, Loading } from '@vben/common-ui';
import { $t } from '#/locales';
import { addClusterApi } from '#/api';

import { useVbenForm, z } from '#/adapter/form';

const [Form, formApi] = useVbenForm({
    handleSubmit: onSubmit,
    schema: [
        {
            component: 'Input',
            componentProps: {
                placeholder: 'Unique name',
            },
            fieldName: 'name',
            label: $t('page.cluster.name'),
            rules: z.string().min(3, { message: 'Enter at least 3 letters' }),
        },
        {
            component: 'VbenInputPassword',
            componentProps: {
                placeholder: 'Security key',
            },
            fieldName: 'key',
            label: $t('page.cluster.key'),
            rules: z.string().min(3, { message: 'Enter at least 3 letters' }),
        },
        {
            component: 'Switch',
            defaultValue: false,
            fieldName: 'enable',
            label: $t('page.site.enable'),
        },
        {
            component: 'Switch',
            defaultValue: false,
            fieldName: 'default_site_enable',
            label: $t('page.site.defaultSiteEnable'),
        },
        {
            component: 'Input',
            componentProps: {
                placeholder: '',
            },
            defaultValue: '',
            fieldName: 'description',
            label: $t('page.cluster.description'),
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

        await addClusterApi(values);
        formApi.resetForm();
        message.success({
            content: `Add cluster successfully!`,
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
        <Card :title="$t('page.cluster.new')">
            <Form />

            <div v-if="submitting" class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-30">
                <Loading :spinning="submitting" />
            </div>
        </Card>
    </Page>
</template>
<script lang="ts" setup>
import { ref } from 'vue';
import { message, Card } from 'ant-design-vue';
import { Page, Loading } from '@vben/common-ui';
import { useVbenForm, z } from '#/adapter/form';
import { useUserStore } from '@vben/stores';
import { useAuthStore } from '#/store';

import { updateUserApi, updateUserPasswordApi } from '#/api';

import { $t } from '#/locales';

const userStore = useUserStore();
const authStore = useAuthStore();

const [Form, formApi] = useVbenForm({
    handleSubmit: onSubmit,
    schema: [
        {
            component: 'VbenInput',
            componentProps: {
                placeholder: $t('authentication.usernameTip'),
                readonly: true,
            },
            fieldName: 'name',
            label: $t('authentication.username'),
            rules: z.string().min(3, { message: 'Enter at least 3 letters' }),
        },
        {
            component: 'VbenInput',
            componentProps: {
                placeholder: $t('authentication.emailTip'),
            },
            fieldName: 'email',
            label: $t('authentication.email'),
            rules: z
                .string()
                .min(1, { message: $t('authentication.emailTip') })
                .email($t('authentication.emailValidErrorTip')),
        }
    ],
    wrapperClass: 'grid-cols-1',
    commonConfig: {
        labelWidth: 200
    },
});
formApi.setValues({ name: userStore.userInfo?.name, email: userStore.userInfo?.email });

const [FormSecurity] = useVbenForm({
    handleSubmit: onSecuritySubmit,
    schema: [
        {
            component: 'VbenInputPassword',
            componentProps: {
                placeholder: $t('authentication.password'),
            },
            fieldName: 'password',
            label: $t('authentication.password'),
            rules: z.string().min(1, { message: $t('authentication.passwordTip') }),
        },
        {
            component: 'VbenInputPassword',
            componentProps: {
                passwordStrength: true,
                placeholder: $t('authentication.password'),
            },
            fieldName: 'new_password',
            label: $t('page.user.password'),
            renderComponentContent() {
                return {
                    strengthText: () => $t('authentication.passwordStrength'),
                };
            },
            rules: z.string().min(1, { message: $t('authentication.passwordTip') }),
        },
        {
            component: 'VbenInputPassword',
            componentProps: {
                placeholder: $t('authentication.confirmPassword'),
            },
            dependencies: {
                rules(values) {
                    const { new_password } = values;
                    return z
                        .string({ required_error: $t('authentication.passwordTip') })
                        .min(1, { message: $t('authentication.passwordTip') })
                        .refine((value) => value === new_password, {
                            message: $t('authentication.confirmPasswordTip'),
                        });
                },
                triggerFields: ['new_password'],
            },
            fieldName: 'confirmPassword',
            label: $t('authentication.confirmPassword'),
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
        submitting.value = true;
        await updateUserApi(userStore.userInfo?.name, values);
        formApi.resetForm();
        message.success({
            content: $t('page.user.updateTip'),
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

async function handleAsyncSecuritySubmit(values: Record<string, any>) {
    try {
        submitting.value = true;

        if (values.password == values.new_password) {
            message.warning({
                content: $t('page.user.passwordUpdateInvalid'),
            });
        }
        else {
            await updateUserPasswordApi(userStore.userInfo?.name, values);
            formApi.resetForm();
            message.success({
                content: $t('page.user.updateTip'),
            });

            await authStore.logout(false);
        }
    } catch {

    } finally {
        submitting.value = false;
    }
}
function onSecuritySubmit(values: Record<string, any>) {
    handleAsyncSecuritySubmit(values).catch(error => {
        console.error('Submit error:', error);
    });

}

</script>

<template>
    <Page content-class="flex flex-col gap-4" description="" title="">
        <Card :title="$t('page.user.basic')">
            <Form />

            <div v-if="submitting" class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-30">
                <Loading :spinning="submitting" />
            </div>
        </Card>

        <Card :title="$t('page.user.security')">
            <FormSecurity />

            <div v-if="submitting" class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-30">
                <Loading :spinning="submitting" />
            </div>
        </Card>
    </Page>
</template>
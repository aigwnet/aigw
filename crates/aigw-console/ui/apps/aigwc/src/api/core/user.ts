import type { UserInfo } from '@vben/types';
import { md5 } from 'js-md5';

import { requestClient } from '#/api/request';


interface ExtUserInfo extends UserInfo {
  email: string,
}

export type { ExtUserInfo };

/**
 * 获取用户信息
 */
export async function getUserInfoApi() {
  return requestClient.get<ExtUserInfo>('/user/info');
}

export async function updateUserApi(name: string, params: Object) {
  return requestClient.put(`/user/profile/${name}`, params);
}

export async function updateUserPasswordApi(name: string, params: Object | any) {
  params.password = md5(params.password);
  params.new_password = md5(params.new_password);
  return requestClient.put(`/user/profile/${name}/password`, params);
}


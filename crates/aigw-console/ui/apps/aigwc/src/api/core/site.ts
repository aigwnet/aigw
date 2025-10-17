
export namespace SiteTableApi {
  export interface PageFetchParams {
    [key: string]: any;
    page: number;
    page_size: number;
  }
}

import { requestClient } from '#/api/request';
import type { BasicResult } from '#/api/core';

export async function getSiteTableApi(cluster: string | null, params: SiteTableApi.PageFetchParams) {
  return requestClient.get(`/sites/page/${cluster}`, { params });
}

export async function addSiteApi(params: Object) {
  return requestClient.post<BasicResult>('/sites', params);
}

export async function updateSiteApi(name: string, params: Object) {
  return requestClient.put(`/sites/${name}`, params);
}

export async function deleteSiteApi(name: string) {
  return requestClient.delete(`/sites/${name}`);
}

export async function getSiteApi(name: string) {
  return requestClient.get(`/sites/${name}`);
}
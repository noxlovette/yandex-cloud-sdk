To retrieve changes from [Yandex's Proto Repo](https://github.com/yandex-cloud/cloudapi/tree/master)

```bash
git remote add cloudapi https://github.com/yandex-cloud/cloudapi.git 
git fetch cloudapi master
git subtree pull --prefix=proto cloudapi master --squash
```
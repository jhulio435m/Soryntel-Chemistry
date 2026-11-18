# Publicación de instaladores con SoryntelDB

Configura estos secretos de GitHub Actions en el fork que publica las versiones:

| Secreto | Valor |
| --- | --- |
| `AWS_ROLE_ARN` | ARN del rol de IAM que asume GitHub Actions mediante OIDC. |
| `AWS_REGION` | Región de AWS del bucket privado. |
| `SORYNTEL_SDB_S3_URI` | URI `s3://<bucket>/<ruta>/soryntel.sdb` del objeto privado. |
| `SORYNTEL_SDB_SHA256` | SHA-256 del archivo, expresado como 64 dígitos hexadecimales. |

El rol de IAM debe confiar en el proveedor OIDC de GitHub Actions, con audiencia `sts.amazonaws.com` y un sujeto limitado al repositorio que publica y a sus referencias `refs/tags/v*`. Concede al rol `s3:GetObject` sobre el objeto privado. Consulta la [configuración de OIDC en AWS](https://docs.github.com/en/actions/how-tos/secure-your-work/security-harden-deployments/oidc-in-aws).

Una ejecución sobre una etiqueta falla si falta un secreto, si AWS rechaza la asunción del rol o si la descarga no pasa la comprobación de integridad. Desde una rama, `workflow_dispatch` emite un aviso y genera paquetes sin la base si falta un secreto o AWS rechaza la asunción. El archivo verificado se distribuye únicamente dentro de los paquetes, junto con su [licencia de datos](../../LICENSE-DATA.md).

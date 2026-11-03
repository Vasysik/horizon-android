#!/bin/bash
# For the owner to run once, signed in to Azure (docs/starling-native-mods.md). Gives the DST VM an identity, and to that
# identity a role that can only start, stop and read the Starling build VM, on that VM alone, so the gateway on the DST VM
# (mod_build_gateway.py) can wake it for a build and put it back to sleep.
set -eu
# Git Bash would take the scope's leading slash for a path of its own.
export MSYS_NO_PATHCONV=1
AZ=${AZ:-"$LOCALAPPDATA/azure-cli/bin/az.cmd"}
SUB=185b4061-8dcc-4c82-a959-b303baa282f0
SCOPE="/subscriptions/$SUB/resourceGroups/starling-build"
VMID="$SCOPE/providers/Microsoft.Compute/virtualMachines/starling-build"
ROLE="Starling build VM power"
FILE=$(mktemp --suffix=.json)
cat > "$FILE" <<EOF
{
  "Name": "$ROLE",
  "Description": "Start, deallocate and read the Starling build VM.",
  "Actions": [
    "Microsoft.Compute/virtualMachines/read",
    "Microsoft.Compute/virtualMachines/start/action",
    "Microsoft.Compute/virtualMachines/deallocate/action"
  ],
  "NotActions": [],
  "AssignableScopes": ["$SCOPE"]
}
EOF
if ! "$AZ" role definition list --name "$ROLE" --query "[0].name" -o tsv | tr -d '\r' | grep -q .; then
  "$AZ" role definition create --role-definition "$(cygpath -w "$FILE")" --query roleName -o tsv | tr -d '\r'
fi
rm -f "$FILE"
P=$("$AZ" vm identity assign -g VMs -n DST --query systemAssignedIdentity -o tsv | tr -d '\r')
echo "DST identity ${P:0:8}..."
for i in $(seq 1 12); do
  "$AZ" role assignment create --assignee-object-id "$P" --assignee-principal-type ServicePrincipal \
    --role "$ROLE" --scope "$VMID" -o none 2>/dev/null || true
  # What Azure holds now, not what the command said.
  if "$AZ" role assignment list --assignee "$P" --scope "$VMID" --query "[].roleDefinitionName" -o tsv \
    | tr -d '\r' | grep -qxF "$ROLE"; then
    echo "DONE"
    exit 0
  fi
  echo "waiting for Azure to see the new role..."
  sleep 15
done
echo "FAILED: the role was not assigned"
exit 1

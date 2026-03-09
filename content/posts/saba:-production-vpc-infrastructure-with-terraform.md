---
title: "saba: production vpc infrastructure with terraform"
description: "building a multi-az aws vpc from scratch with terraform - vpc networking, nat gateways, bastion hosts, and infrastructure as code"
date: 2026-03-09
tags: [terraform, aws, infrastructure]
draft: false
---

## the problem

you need to run workloads on aws. you could click through the console and manually create a vpc, subnets, route tables, security groups - but then what? you can't reproduce it, you can't version it, and you definitely can't tear it down and rebuild it with confidence.

terraform solves this. you describe your infrastructure as code, and terraform figures out how to make reality match your description. saba is a terraform project that provisions a production-ready, multi-az vpc on aws - the kind of network architecture you'd actually use in a real environment.

## terraform fundamentals

before building anything, three concepts matter:

**state** is how terraform tracks what it manages. it maps your config to real aws resources. lose the state file and terraform has no idea those resources belong to it - you'd have to import them manually or risk creating duplicates. this is why remote state backends exist.

**providers** are plugins that teach terraform how to talk to specific apis. terraform core is just an engine - it knows nothing about aws, azure, or anything else until you configure a provider.

**resources vs data sources** - resources are things terraform manages (create, update, delete). data sources are read-only lookups of things that already exist outside your config.

## networking from first principles

### cidr blocks and ip addressing

a cidr block defines a range of ip addresses. `10.0.0.0/16` means the first 16 bits are the network prefix, leaving 16 bits for host addresses - that's 65,536 addresses.

a vpc needs a cidr block to define what ip range is available. subnets carve that range into smaller pieces:

```
VPC: 10.0.0.0/16 (65,536 addresses)
├── public subnet a:  10.0.1.0/24  (256 addresses)
├── public subnet b:  10.0.2.0/24  (256 addresses)
├── private subnet a: 10.0.10.0/24 (256 addresses)
└── private subnet b: 10.0.20.0/24 (256 addresses)
```

### public vs private subnets

the distinction is purely about routing:

- a **public subnet** has a route table that points `0.0.0.0/0` to an internet gateway. resources get public ips and can communicate with the internet bidirectionally.
- a **private subnet** has no route to an internet gateway. resources can't be reached from the internet.

anything that doesn't need to face the public internet belongs in a private subnet - application servers, databases, internal services. this is the most basic form of network isolation.

### routing - igw vs nat gateway

public subnets route to an internet gateway (igw) so the internet can reach them. private subnets route to a nat gateway so they can reach the internet but the internet can't reach them.

the nat gateway translates private ips to its own public ip and only allows responses to outbound requests back in. the traffic flow looks like:

```
public:  Internet ⇄ IGW ⇄ EC2 (public IP)
private: Private EC2 → NAT → IGW → Internet
         Internet ✖→ Private EC2
```

private resources still need outbound internet access - pulling updates, container images, calling external apis. the nat gateway enables this without exposing them to inbound traffic.

a nat gateway needs an elastic ip (eip) because it provides a static public address. if you recreated the nat gateway without one, you'd get a random ip and break any ip-based allowlists.

## building the vpc module

the networking module creates everything: vpc, internet gateway, four subnets across two availability zones, elastic ips, nat gateways, and all the route tables and associations.

the root module orchestrates the child modules:

```hcl
module "networking" {
    source      = "./modules/networking"
    environment = var.environment
    vpc_cidr    = var.vpc_cidr
    az_a        = var.az_a
    az_b        = var.az_b
}

module "bastion" {
    source           = "./modules/bastion"
    environment      = var.environment
    vpc_id           = module.networking.vpc_id
    subnet_id        = module.networking.public_subnet_a_id
    instance_type    = var.instance_type
    public_key_path  = var.public_key_path
    allowed_ssh_cidr = var.allowed_ssh_cidr
}
```

notice how the bastion module references `module.networking.vpc_id` and `module.networking.public_subnet_a_id`. terraform builds a dependency graph from these references and handles sequencing automatically - vpc first, then subnets, then anything that depends on them.

### multi-az for high availability

subnets are placed in two availability zones (us-east-1a and us-east-1b). if one az has an outage, resources in the other az continue running. each az gets its own nat gateway so private subnets aren't sharing a single point of failure:

```
┌─────────────────────┐    ┌─────────────────────┐
│   us-east-1a        │    │   us-east-1b        │
│                     │    │                     │
│  ┌───────────────┐  │    │  ┌───────────────┐  │
│  │  public_a     │  │    │  │  public_b     │  │
│  │  NAT-A + EIP  │  │    │  │  NAT-B + EIP  │  │
│  └───────────────┘  │    │  └───────────────┘  │
│         ▲           │    │         ▲           │
│  ┌───────────────┐  │    │  ┌───────────────┐  │
│  │  private_a    │  │    │  │  private_b    │  │
│  │  routes here  │  │    │  │  routes here  │  │
│  └───────────────┘  │    │  └───────────────┘  │
└─────────────────────┘    └─────────────────────┘
```

## the bastion host

a vpc with no compute is just an empty network. the bastion host is an ec2 instance in the public subnet that acts as a secure jump point to reach private resources.

### security groups

security groups are virtual firewalls attached to individual resources. they're stateful - if you allow inbound traffic on a port, the return traffic is automatically allowed without an explicit outbound rule.

this differs from network acls (nacls), which operate at the subnet level, support both allow and deny rules, and are stateless.

the bastion's security group allows inbound ssh (port 22) and all outbound traffic:

```hcl
resource "aws_security_group" "bastion" {
    name        = "${var.environment}-bastion-sg"
    vpc_id      = var.vpc_id

    ingress {
        from_port   = 22
        to_port     = 22
        protocol    = "tcp"
        cidr_blocks = [var.allowed_ssh_cidr]
    }

    egress {
        from_port   = 0
        to_port     = 0
        protocol    = "-1"
        cidr_blocks = ["0.0.0.0/0"]
    }
}
```

in production, you'd restrict `allowed_ssh_cidr` to your ip rather than `0.0.0.0/0`.

### dynamic ami lookup

rather than hardcoding an ami id (which varies by region and changes over time), the bastion uses a data source to find the latest amazon linux 2023 image dynamically:

```hcl
data "aws_ami" "bastion" {
    most_recent = true
    owners      = [var.ami_owner]

    filter {
        name   = "name"
        values = [var.ami_name_filter]
    }
}
```

this also makes it easy to swap operating systems by overriding the variables:

```bash
# amazon linux 2023 (default)
terraform apply

# ubuntu 24.04
terraform apply \
    -var="ami_name_filter=ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*" \
    -var="ami_owner=099720109477"
```

### connecting

ssh into the bastion, then jump to private resources:

```bash
# direct connection to bastion
ssh -i ~/.ssh/bastion-key ec2-user@$(terraform output -raw bastion_public_ip)

# jump through bastion to a private instance
ssh -i ~/.ssh/bastion-key -J ec2-user@<bastion-ip> ec2-user@<private-ip>
```

## the terraform lifecycle

running `plan`, `apply`, and `destroy` is the full lifecycle. a few things stood out while working through it.

**dependency resolution is automatic.** terraform inferred that the subnet depends on the vpc from `vpc_id = aws_vpc.main.id`. on create, it builds the vpc first. on destroy, it tears down the subnet first. you never specify ordering manually.

**`(known after apply)` values** are things aws generates at creation time - arns, ids, availability zones. they can't be known until the resource exists, so terraform marks them as pending.

**state is everything.** after `apply`, terraform writes a `terraform.tfstate` file. this is how it knows what to update or destroy. the state file is the single source of truth for what terraform manages.

**credentials matter early.** my first `terraform plan` failed because i was logged into the terraform cli but not aws. the error was clear enough - no valid credential sources found. logging into aws fixed it immediately.

## what i learned

this project was about understanding vpc networking from first principles rather than clicking through the aws console. the key takeaways:

- network isolation is just routing. public vs private is determined by whether a route table points to an igw or a nat gateway
- nat gateways are the bridge that lets private resources reach the internet without being reachable from it
- multi-az is about eliminating single points of failure, including having one nat gateway per az
- security groups are stateful firewalls at the resource level. nacls are stateless firewalls at the subnet level
- terraform's dependency graph handles sequencing automatically from resource references
- state files are critical infrastructure - treat them accordingly

the source code is at [github.com/aidantrabs/saba](https://github.com/aidantrabs/saba).

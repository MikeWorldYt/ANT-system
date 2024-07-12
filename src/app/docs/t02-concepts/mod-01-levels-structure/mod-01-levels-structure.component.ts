import { Component } from '@angular/core';
import { HeaderT02Component } from '../header-t02/header-t02.component';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'doc-t02-mod-01-levels-structure',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule
  ],
  templateUrl: './mod-01-levels-structure.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod01_Component {
  hovered = false;

showHash(event: Event) {
  this.hovered = true;
}

hideHash(event: Event) {
  this.hovered = false;
}
}
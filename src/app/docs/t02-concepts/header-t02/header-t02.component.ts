import { Component, Input } from '@angular/core';

@Component({
  selector: 'header-t02',
  standalone: true,
  imports: [],
  templateUrl: './header-t02.component.html',
  styleUrl: '../../docs.component.css'
})
export class HeaderT02Component {
  @Input() mod: string = '';
}
